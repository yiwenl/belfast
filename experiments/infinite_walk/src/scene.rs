use belfast::{BelfastResult, Buffer, BufferUsage, Device, PerspectiveCamera};

use crate::{
    input::SkeletonInput,
    point_cloud::PointCloud,
    walk::{Walk, DRIFT_X, LENGTH, START},
};

pub const SHADER: &str = include_str!("points.wgsl");

pub struct Scene {
    pub camera: PerspectiveCamera,
    pub walk: Walk,
    pub skeleton: SkeletonInput,
    pipeline: wgpu::RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    points: Option<Buffer>,
    point_count: u32,
    point_size: f32,
    white_particles: bool,
    camera_y: f32,
    light_forward: f32,
    light_height: f32,
    light_intensity: f32,
    light_range: f32,
    viewport: [u32; 2],
    depth: Option<(wgpu::Texture, wgpu::TextureView)>,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("InfiniteWalkCameraLayout"),
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
                    label: Some("InfiniteWalkLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("InfiniteWalkPoints"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("InfiniteWalkOpaqueCircles"),
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
        let camera_buffer =
            Buffer::create(device, 144, BufferUsage::uniform(), "InfiniteWalkCamera");
        let camera_bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InfiniteWalkCamera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.gpu().as_entire_binding(),
            }],
        });
        Ok(Self {
            camera: PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0),
            walk: Walk::default(),
            skeleton: SkeletonInput::default(),
            pipeline,
            camera_buffer,
            camera_bind_group,
            points: None,
            point_count: 0,
            point_size: 0.01,
            white_particles: false,
            camera_y: START[1],
            light_forward: 3.5,
            light_height: 2.0,
            light_intensity: 12.0,
            light_range: 3.3,
            viewport: [0, 0],
            depth: None,
        })
    }

    pub fn load_point_cloud(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let cloud = PointCloud::parse(bytes)?;
        let count = cloud.points.len() as u32;
        if u64::from(count) * 16 > device.gpu().limits().max_buffer_size {
            return Err("Point cloud exceeds this GPU's vertex buffer limit".into());
        }
        self.points = Some(Buffer::from_data(
            device,
            &cloud.points,
            BufferUsage::vertex(),
            "InfiniteWalkPoints",
        ));
        self.point_count = count;
        Ok(count)
    }

    pub fn set_point_size(&mut self, size: f32) -> Result<(), String> {
        if !size.is_finite() || !(0.001..=0.05).contains(&size) {
            return Err("Particle size must be between 0.001 and 0.05 world units".into());
        }
        self.point_size = size;
        Ok(())
    }

    pub fn set_camera_y(&mut self, y: f32) -> Result<(), String> {
        if !y.is_finite() || !(-2.0..=3.0).contains(&y) {
            return Err("Camera Y must be between -2 and 3 world units".into());
        }
        self.camera_y = y;
        Ok(())
    }

    pub fn set_white_particles(&mut self, enabled: bool) {
        self.white_particles = enabled;
    }

    pub fn set_light(
        &mut self,
        forward: f32,
        height: f32,
        intensity: f32,
        range: f32,
    ) -> Result<(), String> {
        if !(0.0..=10.0).contains(&forward)
            || !(0.1..=5.0).contains(&height)
            || !(0.0..=30.0).contains(&intensity)
            || !(0.2..=10.0).contains(&range)
        {
            return Err("Invalid spotlight offset, height, intensity or range".into());
        }
        self.light_forward = forward;
        self.light_height = height;
        self.light_intensity = intensity;
        self.light_range = range;
        Ok(())
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32, _pixel_scale: f32) {
        if self.viewport == [width, height] {
            return;
        }
        self.viewport = [width, height];
        let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("InfiniteWalkDepth"),
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

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        self.walk.advance(delta_seconds);
        let mut eye = self.walk.position();
        eye[1] = self.camera_y;
        self.camera
            .look_at(eye, [eye[0] + DRIFT_X, eye[1], eye[2] - LENGTH]);
        let mut uniform = [0.0_f32; 36];
        let projection = self.camera.projection_matrix();
        uniform[..16].copy_from_slice(&self.camera.view_projection_matrix());
        uniform[16..20].copy_from_slice(&[
            projection[0],
            projection[5],
            self.point_size,
            if device.format().is_srgb() { 1.0 } else { 0.0 },
        ]);
        uniform[20..24].copy_from_slice(&[eye[2], 1.0, LENGTH, DRIFT_X]);
        uniform[24] = if self.white_particles { 1.0 } else { 0.0 };
        let forward_scale = self.light_forward / DRIFT_X.hypot(LENGTH);
        uniform[28..32].copy_from_slice(&[
            eye[0] + DRIFT_X * forward_scale,
            eye[1] + self.light_height,
            eye[2] - LENGTH * forward_scale,
            self.light_intensity,
        ]);
        // Fixed downward cone: full strength inside 30°, fading to zero at 45°.
        uniform[32..36].copy_from_slice(&[
            self.light_range,
            30.0_f32.to_radians().cos(),
            45.0_f32.to_radians().cos(),
            0.025,
        ]);
        self.camera_buffer.write(device, &uniform, 0);
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&self, device: &Device, view: &wgpu::TextureView) {
        let Some((_, depth)) = &self.depth else {
            return;
        };
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("InfiniteWalkEncoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("InfiniteWalkPoints"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.025,
                            g: 0.025,
                            b: 0.025,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(points) = &self.points {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, points.gpu().slice(..));
                pass.draw(0..6, 0..self.point_count);
            }
        }
        device.queue().submit([encoder.finish()]);
    }
}
