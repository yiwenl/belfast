use crate::model_data::{ImageData, ModelData, Vertex};
use crate::shadow::{ShadowMap, DEPTH_SHADER, SAMPLING_SHADER};
use belfast::Device;
use glam::Mat4;
use wgpu::util::DeviceExt;

pub const SHADER: &str = include_str!("model.wgsl");
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub fn shader_source() -> String {
    format!(
        "{}\n{SAMPLING_SHADER}\n{SHADER}",
        belfast::WGSL_SHADOW_PCF_3X3
    )
}

#[derive(Clone, Copy, Debug)]
pub struct PointLight {
    pub position: [f32; 3],
    /// Linear RGB, independent of the yellow position marker.
    pub color: [f32; 3],
    pub intensity: f32,
}

struct Mesh {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    binding: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

pub struct ModelRenderer {
    meshes: Vec<Mesh>,
    shadow_binding: wgpu::BindGroup,
    shadow_depth_binding: wgpu::BindGroup,
    empty_instances: wgpu::BindGroup,
}

impl ModelRenderer {
    pub fn new(device: &Device, data: &ModelData) -> Result<Self, String> {
        Self::build(
            device,
            data,
            None,
            &shader_source(),
            "vs_main",
            DEPTH_SHADER,
            "vs_shadow",
            None,
        )
    }

    pub fn new_shadowed(
        device: &Device,
        data: &ModelData,
        shadow: &ShadowMap,
    ) -> Result<Self, String> {
        Self::build(
            device,
            data,
            None,
            &shader_source(),
            "vs_main",
            DEPTH_SHADER,
            "vs_shadow",
            Some(shadow),
        )
    }

    pub(crate) fn new_instanced(
        device: &Device,
        data: &ModelData,
        instance_layout: &wgpu::BindGroupLayout,
        instance_shader: &str,
        depth_shader: &str,
        shadow: Option<&ShadowMap>,
    ) -> Result<Self, String> {
        // Reuse the same material and fragment lighting, including live shader edits.
        Self::build(
            device,
            data,
            Some(instance_layout),
            &format!("{}\n{instance_shader}", shader_source()),
            "vs_instances",
            depth_shader,
            "vs_shadow_instances",
            shadow,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        device: &Device,
        data: &ModelData,
        instance_layout: Option<&wgpu::BindGroupLayout>,
        source: &str,
        vertex_entry: &str,
        depth_source: &str,
        depth_entry: &str,
        shadow: Option<&ShadowMap>,
    ) -> Result<Self, String> {
        let gpu = device.gpu();
        let fallback = if shadow.is_none() {
            Some(ShadowMap::new(device, 1, Mat4::IDENTITY, 0.).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let shadow = shadow.or(fallback.as_ref()).unwrap();
        let layout = gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ModelLayout"),
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
                texture_entry(1),
                texture_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let empty_layout = gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("EmptyInstancesLayout"),
            entries: &[],
        });
        let empty_instances = gpu.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("EmptyInstancesBinding"),
            layout: &empty_layout,
            entries: &[],
        });
        let layouts = [
            Some(&layout),
            Some(instance_layout.unwrap_or(&empty_layout)),
            Some(&shadow.sample_layout),
        ];
        let pipeline_layout = gpu.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ModelPipelineLayout"),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let shader = gpu.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ModelShader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let shadow_layout = gpu.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ModelShadowPipelineLayout"),
            bind_group_layouts: &[
                Some(&shadow.depth_layout),
                Some(instance_layout.unwrap_or(&empty_layout)),
            ],
            immediate_size: 0,
        });
        let shadow_shader = gpu.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ModelShadowShader"),
            source: wgpu::ShaderSource::Wgsl(depth_source.into()),
        });
        let sampler = gpu.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("BarkSampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mut meshes = Vec::new();
        for mesh in &data.primitives {
            if (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as u64
                > gpu.limits().max_buffer_size
                || mesh.indices.len() as u64 * 4 > gpu.limits().max_buffer_size
            {
                return Err("GLB mesh exceeds GPU buffer limits".into());
            }
            let base = upload_image(
                device,
                &mesh.base_color,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )?;
            let normal = upload_image(device, &mesh.normal_map, wgpu::TextureFormat::Rgba8Unorm)?;
            let mut values = Mat4::IDENTITY.to_cols_array().to_vec();
            values.extend(mesh.base_factor);
            values.extend([
                mesh.normal_scale,
                u32::from(device.format().is_srgb()) as f32,
                0.,
                0.,
            ]);
            // Point light starts disabled until configured by the scene.
            values.extend([0.; 8]);
            let uniform = gpu.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ModelUniform"),
                contents: bytemuck::cast_slice(&values),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let binding = gpu.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ModelBinding"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&base),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let pipeline = gpu.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ModelPipeline"), layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader, entry_point: Some(vertex_entry), compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format: device.format(), blend: None, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState { cull_mode: if mesh.double_sided { None } else { Some(wgpu::Face::Back) }, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(), bias: Default::default(),
                }),
                multisample: Default::default(), multiview_mask: None, cache: None,
            });
            let vertices = gpu.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ModelVertices"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let shadow_pipeline = gpu.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ModelShadowDepth"), layout: Some(&shadow_layout),
                vertex: wgpu::VertexState {
                    module: &shadow_shader, entry_point: Some(depth_entry), compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                    })],
                },
                fragment: None,
                // Thin petals and branches must cast from either side.
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(), bias: wgpu::DepthBiasState { constant: 2, slope_scale: 1., clamp: 0. },
                }),
                multisample: Default::default(), multiview_mask: None, cache: None,
            });
            let indices = gpu.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ModelIndices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            meshes.push(Mesh {
                pipeline,
                shadow_pipeline,
                binding,
                uniform,
                vertices,
                indices,
                count: mesh.indices.len() as u32,
            });
        }
        Ok(Self {
            meshes,
            shadow_binding: shadow.sample_binding.clone(),
            shadow_depth_binding: shadow.depth_binding.clone(),
            empty_instances,
        })
    }

    pub fn update_camera(&self, device: &Device, view_projection: &[f32; 16]) {
        for mesh in &self.meshes {
            device
                .queue()
                .write_buffer(&mesh.uniform, 0, bytemuck::cast_slice(view_projection));
        }
    }

    pub fn update_light(&self, device: &Device, light: PointLight) {
        let values = [
            light.position[0],
            light.position[1],
            light.position[2],
            light.intensity,
            light.color[0],
            light.color[1],
            light.color[2],
            0.,
        ];
        for mesh in &self.meshes {
            device
                .queue()
                .write_buffer(&mesh.uniform, 96, bytemuck::cast_slice(&values));
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.draw_instances(pass, None, 1);
    }

    pub(crate) fn draw_instances<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        instances: Option<&'a wgpu::BindGroup>,
        count: u32,
    ) {
        for mesh in &self.meshes {
            pass.set_pipeline(&mesh.pipeline);
            pass.set_bind_group(0, &mesh.binding, &[]);
            pass.set_bind_group(1, instances.unwrap_or(&self.empty_instances), &[]);
            pass.set_bind_group(2, &self.shadow_binding, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.count, 0, 0..count);
        }
    }

    pub fn draw_shadow<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.draw_shadow_instances(pass, None, 1);
    }

    pub(crate) fn draw_shadow_instances<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        instances: Option<&'a wgpu::BindGroup>,
        count: u32,
    ) {
        for mesh in &self.meshes {
            pass.set_pipeline(&mesh.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_depth_binding, &[]);
            pass.set_bind_group(1, instances.unwrap_or(&self.empty_instances), &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.count, 0, 0..count);
        }
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn upload_image(
    device: &Device,
    image: &ImageData,
    format: wgpu::TextureFormat,
) -> Result<wgpu::TextureView, String> {
    let limit = device.gpu().limits().max_texture_dimension_2d;
    if image.width == 0
        || image.height == 0
        || image.width > limit
        || image.height > limit
        || u64::from(image.width) * u64::from(image.height) * 4 != image.rgba.len() as u64
    {
        return Err("GLB texture dimensions exceed GPU limits or have invalid RGBA data".into());
    }
    let size = wgpu::Extent3d {
        width: image.width,
        height: image.height,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("ModelTexture"),
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
        &image.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width * 4),
            rows_per_image: Some(image.height),
        },
        size,
    );
    Ok(texture.create_view(&Default::default()))
}

pub fn depth_view(device: &Device, viewport: [u32; 2]) -> wgpu::TextureView {
    device
        .gpu()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("CourtyardDepth"),
            size: wgpu::Extent3d {
                width: viewport[0].max(1),
                height: viewport[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
