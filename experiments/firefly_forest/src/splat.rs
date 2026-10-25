use crate::{ply::GaussianCloud, sorting::SortCamera};
use belfast::Device;

pub const SHADER: &str = include_str!("splat.wgsl");
const BRUSH_SIZE: u32 = 256;
const BRUSH_LAYERS: u32 = 4;
const BRUSH_MIPS: u32 = 9;

pub struct SplatRenderer {
    pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    camera: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    visible: u32,
    srgb: bool,
    brushes: wgpu::Texture,
    brush_enabled: bool,
    brush_ready: bool,
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
        let brushes = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("ForestBrushMasks"),
            size: wgpu::Extent3d {
                width: BRUSH_SIZE,
                height: BRUSH_SIZE,
                depth_or_array_layers: BRUSH_LAYERS,
            },
            mip_level_count: BRUSH_MIPS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let brush_sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ForestBrushSampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
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
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&brushes.create_view(
                        &wgpu::TextureViewDescriptor {
                            dimension: Some(wgpu::TextureViewDimension::D2Array),
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&brush_sampler),
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
            brushes,
            brush_enabled: false,
            brush_ready: false,
        })
    }

    pub fn set_brush_enabled(&mut self, enabled: bool) {
        self.brush_enabled = enabled;
    }

    pub fn upload_brush_masks(&mut self, device: &Device, pixels: &[u8]) -> Result<(), String> {
        let layer_size = (BRUSH_SIZE * BRUSH_SIZE) as usize;
        if pixels.len() != layer_size * BRUSH_LAYERS as usize {
            return Err("Expected four 256 × 256 brush alpha masks".into());
        }
        for layer in 0..BRUSH_LAYERS {
            let offset = layer as usize * layer_size;
            let mut data = pixels[offset..offset + layer_size].to_vec();
            let mut size = BRUSH_SIZE;
            for mip in 0..BRUSH_MIPS {
                device.queue().write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.brushes,
                        mip_level: mip,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size),
                        rows_per_image: Some(size),
                    },
                    wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                );
                if size == 1 {
                    break;
                }
                let next = size / 2;
                data = (0..next * next)
                    .map(|i| {
                        let p = ((i / next * 2) * size + i % next * 2) as usize;
                        ((u32::from(data[p])
                            + u32::from(data[p + 1])
                            + u32::from(data[p + size as usize])
                            + u32::from(data[p + size as usize + 1])
                            + 2)
                            / 4) as u8
                    })
                    .collect();
                size = next;
            }
        }
        self.brush_ready = true;
        Ok(())
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
        data.extend([
            u32::from(self.srgb) as f32,
            u32::from(self.brush_enabled && self.brush_ready) as f32,
        ]);
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
