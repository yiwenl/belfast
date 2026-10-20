use crate::{ply::GaussianCloud, sorting::SortCamera};
use belfast::Device;

pub const SHADER: &str = include_str!("splat.wgsl");

pub struct SplatRenderer {
    pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    camera: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    visible: u32,
    srgb: bool,
}

impl SplatRenderer {
    pub fn new(device: &Device, cloud: &mut GaussianCloud) -> Result<Self, String> {
        Self::new_with_depth(device, cloud, None)
    }

    pub fn new_with_depth(
        device: &Device,
        cloud: &mut GaussianCloud,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Result<Self, String> {
        let count = cloud.positions.len() as u32;
        let limit = device.gpu().limits().max_texture_dimension_2d;
        let width = limit.min(2048).min(count);
        let height = count.div_ceil(width);
        if height > limit {
            return Err(format!(
                "{count} Gaussians exceed this GPU's texture capacity"
            ));
        }
        if u64::from(count) * 4 > device.gpu().limits().max_buffer_size {
            return Err("Gaussian index buffer exceeds GPU limits".into());
        }
        let padded = (width * height) as usize;
        cloud.positions.resize(padded, [0.; 4]);
        cloud.attributes.resize(padded, [0; 4]);
        let position_texture = upload_texture(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba32Float,
            bytemuck::cast_slice(&cloud.positions),
            "GaussianPositions",
        );
        let attribute_texture = upload_texture(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba32Uint,
            bytemuck::cast_slice(&cloud.attributes),
            "GaussianAttributes",
        );
        // The GPU owns immutable attributes from here; no per-frame attribute uploads.
        cloud.positions = Vec::new();
        cloud.attributes = Vec::new();
        let camera = device.gpu().create_buffer(&wgpu::BufferDescriptor {
            label: Some("GaussianCamera"),
            size: 144,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let indices = device.gpu().create_buffer(&wgpu::BufferDescriptor {
            label: Some("GaussianSortedIndices"),
            size: u64::from(count) * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GaussianLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GaussianBindGroup"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &position_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &attribute_texture.create_view(&Default::default()),
                    ),
                },
            ],
        });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("GaussianPipelineLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("GaussianShader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let mut targets = vec![Some(wgpu::ColorTargetState {
            format: device.format(),
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        if crate::fireflies::merged_depth(device) {
            let min = wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Min,
            };
            targets.push(Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R32Float,
                blend: Some(wgpu::BlendState {
                    color: min,
                    alpha: min,
                }),
                write_mask: wgpu::ColorWrites::RED,
            }));
        }
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("GaussianPipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 4,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 0,
                            shader_location: 0,
                        }],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if crate::fireflies::merged_depth(device) {
                        "fs_merged"
                    } else {
                        "fs_main"
                    }),
                    compilation_options: Default::default(),
                    targets: &targets,
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        let depth_pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ForestOcclusionPipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_depth"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 4,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 0,
                            shader_location: 0,
                        }],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_depth"),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::fireflies::DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        Ok(Self {
            pipeline,
            depth_pipeline,
            bind_group,
            camera,
            indices,
            count,
            visible: 0,
            srgb: device.format().is_srgb(),
        })
    }

    pub fn set_indices(&mut self, device: &Device, indices: &[u32]) -> Result<(), String> {
        if indices.len() > self.count as usize || indices.iter().any(|&i| i >= self.count) {
            return Err("Invalid Gaussian sort indices".into());
        }
        if !indices.is_empty() {
            device
                .queue()
                .write_buffer(&self.indices, 0, bytemuck::cast_slice(indices));
        }
        self.visible = indices.len() as u32;
        Ok(())
    }

    pub fn update_camera(&self, device: &Device, camera: &SortCamera) {
        let mut data = camera.encode();
        data.extend([u32::from(self.srgb) as f32, 0.]);
        device
            .queue()
            .write_buffer(&self.camera, 0, bytemuck::cast_slice(&data));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.indices.slice(..));
        pass.draw(0..4, 0..self.visible);
    }

    /// Approximate opaque forest coverage without changing splat color blending.
    pub fn draw_depth<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.depth_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.indices.slice(..));
        pass.draw(0..4, 0..self.visible);
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn visible(&self) -> u32 {
        self.visible
    }
}

fn upload_texture(
    device: &Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    bytes: &[u8],
    label: &str,
) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    device.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 16),
            rows_per_image: Some(height),
        },
        size,
    );
    texture
}
