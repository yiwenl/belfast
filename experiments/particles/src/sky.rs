use belfast::Device;
use glam::Mat4;

pub const SHADER: &str = include_str!("sky.wgsl");

/// Inverse projection followed by camera rotation, never camera translation.
pub fn ray_matrix(view: Mat4, projection: Mat4) -> Mat4 {
    let rotation = Mat4::from_mat3(glam::Mat3::from_mat4(view));
    rotation.transpose() * projection.inverse()
}

pub fn validate_image(width: u32, height: u32, bytes: usize, limit: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > limit
        || height > limit
        || height.checked_mul(2) != Some(width)
    {
        return Err(format!(
            "Sky panorama must be 2:1 and at most {limit} pixels wide"
        ));
    }
    if u64::from(width) * u64::from(height) * 4 != bytes as u64 {
        return Err("Sky panorama must contain exactly width × height × 4 RGBA bytes".into());
    }
    Ok(())
}

/// Opaque infinite-distance background: one triangle, no splats and no sorting.
pub struct SkyRenderer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    mode: u32,
    has_image: bool,
    srgb: bool,
}

impl SkyRenderer {
    pub fn new(device: &Device) -> Self {
        let gpu = device.gpu();
        let uniform = gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SkyCamera"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SkyLayout"),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = gpu.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SkySampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = image_binding(device, &layout, &uniform, &sampler, 2, 1, &[0; 8]);
        let shader = gpu.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SkyShader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = gpu.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SkyPipelineLayout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = gpu.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SkyPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
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
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            layout,
            bind_group,
            uniform,
            sampler,
            mode: 0,
            has_image: false,
            srgb: device.format().is_srgb(),
        }
    }

    pub fn set_preset(&mut self, preset: &str) -> Result<(), String> {
        self.mode = match preset {
            "day" => 0,
            "night" => 1,
            "image" if self.has_image => 2,
            "off" => 3,
            _ => {
                return Err(
                    "Sky preset must be day, night, off, or image after loading a panorama".into(),
                )
            }
        };
        Ok(())
    }

    pub fn set_image(
        &mut self,
        device: &Device,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        validate_image(
            width,
            height,
            rgba.len(),
            device.gpu().limits().max_texture_dimension_2d,
        )?;
        self.bind_group = image_binding(
            device,
            &self.layout,
            &self.uniform,
            &self.sampler,
            width,
            height,
            rgba,
        );
        self.has_image = true;
        self.mode = 2;
        Ok(())
    }

    pub fn update(&self, device: &Device, view: Mat4, projection: Mat4) {
        let mut values = ray_matrix(view, projection).to_cols_array().to_vec();
        values.extend([self.mode as f32, u32::from(self.srgb) as f32, 0., 0.]);
        device
            .queue()
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&values));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.mode == 3 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn image_binding(
    device: &Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("SkyPanorama"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
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
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        size,
    );
    device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("SkyBindGroup"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
