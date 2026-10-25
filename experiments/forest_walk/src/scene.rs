use crate::{
    camera_path::{CameraNavigator, CameraTrack},
    point_cloud::PointCloud,
};
use belfast::{BelfastResult, Buffer, BufferUsage, Device};
use glam::Mat4;

pub const SHADER: &str = include_str!("points.wgsl");

pub struct Scene {
    pub navigation: Option<CameraNavigator>,
    pipeline: wgpu::RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    points: Option<Buffer>,
    point_count: u32,
    point_size: f32,
    viewport: [u32; 2],
    pixel_scale: f32,
    depth: Option<(wgpu::Texture, wgpu::TextureView)>,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ForestWalkCameraLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ForestWalkLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ForestWalkPoints"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ForestWalkOpaqueCircles"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 16,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Unorm8x4,
                                offset: 12,
                                shader_location: 1,
                            },
                        ],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: device.format(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let camera_buffer = Buffer::create(device, 80, BufferUsage::uniform(), "ForestWalkCamera");
        let camera_bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ForestWalkCamera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.gpu().as_entire_binding(),
            }],
        });
        Ok(Self {
            navigation: None,
            pipeline,
            camera_buffer,
            camera_bind_group,
            points: None,
            point_count: 0,
            point_size: 2.,
            viewport: [1, 1],
            pixel_scale: 1.,
            depth: None,
        })
    }

    pub fn load_point_cloud(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let cloud = PointCloud::parse(bytes)?;
        let count = cloud.points.len() as u32;
        if u64::from(count) * 16 > device.gpu().limits().max_buffer_size {
            return Err("Point cloud exceeds this GPU's vertex buffer limit".into());
        }
        let points = Buffer::from_data(
            device,
            &cloud.points,
            BufferUsage::vertex(),
            "ForestWalkPoints",
        );
        self.points = Some(points);
        self.point_count = count;
        Ok(count)
    }

    pub fn load_cameras(&mut self, json: &str) -> Result<u32, String> {
        let track = CameraTrack::from_json(json)?;
        let count = track.len() as u32;
        self.navigation = Some(CameraNavigator::new(track));
        Ok(count)
    }

    pub fn set_point_size(&mut self, pixels: f32) -> Result<(), String> {
        if !pixels.is_finite() || !(0.5..=12.).contains(&pixels) {
            return Err("Point size must be between 0.5 and 12 CSS pixels".into());
        }
        self.point_size = pixels;
        Ok(())
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32, pixel_scale: f32) {
        self.pixel_scale = pixel_scale;
        if self.depth.is_some() && self.viewport == [width, height] {
            return;
        }
        self.viewport = [width, height];
        let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("ForestWalkDepth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        self.depth = Some((texture, view));
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, _now_ms: f64) {
        let Some(navigation) = &mut self.navigation else {
            return;
        };
        navigation.advance(delta_seconds);
        let pose = navigation.pose();
        let aspect = self.viewport[0] as f32 / self.viewport[1] as f32;
        let projection = Mat4::perspective_rh(pose.fov_y, aspect, 0.005, 1000.);
        let mut uniform = [0f32; 20];
        uniform[..16].copy_from_slice(&(projection * pose.view_matrix()).to_cols_array());
        uniform[16..].copy_from_slice(&[
            self.viewport[0] as f32,
            self.viewport[1] as f32,
            self.point_size * self.pixel_scale,
            if device.format().is_srgb() { 1. } else { 0. },
        ]);
        self.camera_buffer.write(device, &uniform, 0);
    }

    pub fn render(&self, device: &Device, view: &wgpu::TextureView) {
        let Some((_, depth)) = &self.depth else {
            return;
        };
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ForestWalkEncoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ForestWalkPoints"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.012,
                            g: 0.015,
                            b: 0.012,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(points) = &self.points {
                if self.navigation.is_some() {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_vertex_buffer(0, points.gpu().slice(..));
                    pass.draw(0..6, 0..self.point_count);
                }
            }
        }
        device.queue().submit([encoder.finish()]);
    }
}
