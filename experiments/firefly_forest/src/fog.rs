use crate::{fireflies::merged_depth, sorting::SortCamera};
use belfast::Device;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub(crate) const PARAMS_SHADER: &str = include_str!("fog-params.wgsl");
const SCALE: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    inverse_camera: [f32; 16],
    eye_time: [f32; 4],
    controls: [f32; 4],
    light_position: [f32; 4],
    light_color: [f32; 4],
}

pub struct Fog {
    pub view: wgpu::TextureView,
    pub distance: wgpu::TextureView,
    pub params: wgpu::Buffer,
    values: Params,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    binding: wgpu::BindGroup,
    noise: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl Fog {
    pub fn new(device: &Device, depth: &wgpu::TextureView) -> Self {
        let values = Params {
            inverse_camera: glam::Mat4::IDENTITY.to_cols_array(),
            eye_time: [0.; 4],
            controls: [0.06, 1., 0.7, 1.],
            light_position: [0., 2.2, -10., 32.],
            light_color: [0.30, 0.55, 0.95, 0.45],
        };
        let params = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FogParams"),
                contents: bytemuck::bytes_of(&values),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let noise_texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("FogDensityNoise"),
            size: wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        // One small seeded noise volume; animated sampling never uploads density.
        let mut seed = 0x9e3779b9u32;
        let data: Vec<u8> = (0..32 * 32 * 32)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed >> 24) as u8
            })
            .collect();
        device.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &noise_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(32),
                rows_per_image: Some(32),
            },
            wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 32,
            },
        );
        let noise = noise_texture.create_view(&Default::default());
        let sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("FogNoiseSampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("FogLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: if merged_depth(device) { 1 } else { 4 },
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: if merged_depth(device) {
                                wgpu::TextureSampleType::Float { filterable: false }
                            } else {
                                wgpu::TextureSampleType::Depth
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VolumetricFogShader"),
                source: wgpu::ShaderSource::Wgsl(
                    format!("{PARAMS_SHADER}\n{}", include_str!("fog.wgsl")).into(),
                ),
            });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FogPipelineLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VolumetricFog"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if merged_depth(device) {
                        "fs_main"
                    } else {
                        "fs_depth"
                    }),
                    compilation_options: Default::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::R32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::RED,
                        }),
                    ],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let (view, distance) = Self::targets(device, [1, 1]);
        let binding = Self::bind(device, &layout, &params, depth, &noise, &sampler);
        Self {
            view,
            distance,
            params,
            values,
            pipeline,
            layout,
            binding,
            noise,
            sampler,
        }
    }

    fn targets(device: &Device, viewport: [u32; 2]) -> (wgpu::TextureView, wgpu::TextureView) {
        let target = |format, label| {
            device
                .gpu()
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: viewport[0].div_ceil(SCALE).max(1),
                        height: viewport[1].div_ceil(SCALE).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        (
            target(wgpu::TextureFormat::Rgba16Float, "FogScattering"),
            target(wgpu::TextureFormat::R32Float, "FogDistance"),
        )
    }

    fn bind(
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        params: &wgpu::Buffer,
        depth: &wgpu::TextureView,
        noise: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FogBindings"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: if merged_depth(device) { 1 } else { 4 },
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(noise),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn resize(&mut self, device: &Device, viewport: [u32; 2], depth: &wgpu::TextureView) {
        (self.view, self.distance) = Self::targets(device, viewport);
        self.binding = Self::bind(
            device,
            &self.layout,
            &self.params,
            depth,
            &self.noise,
            &self.sampler,
        );
    }

    pub fn configure(
        &mut self,
        enabled: bool,
        density: f32,
        speed: f32,
        light: f32,
    ) -> Result<(), String> {
        if !(0.0..=0.2).contains(&density)
            || !(0.0..=2.0).contains(&speed)
            || !(0.0..=4.0).contains(&light)
        {
            return Err("Invalid fog settings".into());
        }
        self.values.controls = [density, speed, light, if enabled { 1. } else { 0. }];
        Ok(())
    }

    pub fn reset(&mut self) {
        self.values.eye_time[3] = 0.;
    }

    pub fn update(&mut self, device: &Device, camera: &SortCamera, dt: f32) {
        self.values.inverse_camera = (camera.projection * camera.view).inverse().to_cols_array();
        let eye = camera.view.inverse().w_axis;
        self.values.eye_time[..3].copy_from_slice(&[eye.x, eye.y, eye.z]);
        self.values.eye_time[3] += dt.clamp(0., 0.1) * self.values.controls[1];
        device
            .queue()
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&self.values));
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        start: Option<&wgpu::QuerySet>,
    ) -> bool {
        if self.values.controls[3] == 0. {
            return false;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("VolumetricFog"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.,
                            g: 0.,
                            b: 0.,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.distance,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: start.map(|query_set| wgpu::RenderPassTimestampWrites {
                query_set,
                beginning_of_pass_write_index: Some(8),
                end_of_pass_write_index: None,
            }),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}
