use crate::{
    fluid_debug::FluidDebug,
    particles::{Particles, COUNT, STATE},
};
use belfast::{
    fluid::FluidSimulation, BelfastResult, BindGroup, Buffer, BufferUsage, Device, Draw,
    DrawOptions, Geom, Mesh, MeshIndexFormat, PerspectiveCamera, Texture, TextureOptions,
    VertexAttributeDescriptor, VertexBufferBinding,
};
use glam::{Mat4, Vec3};
pub const BACKGROUND: &str = include_str!("background.wgsl");
pub fn shader_source() -> String {
    format!(
        "{STATE}\n{}\n{}",
        belfast::WGSL_SHADOW_PCF_3X3,
        include_str!("blocks.wgsl")
    )
}
const SHADOW_SIZE: u32 = 2048;
pub struct Renderer {
    mesh: Mesh,
    draw: Draw,
    shadow_pipeline: wgpu::RenderPipeline,
    globals: Buffer,
    globals_group: BindGroup,
    particle_groups: [BindGroup; 2],
    material_group: BindGroup,
    shadow: Texture,
    depth: Texture,
    background: Draw,
    background_mesh: Mesh,
    background_group: BindGroup,
    fluid_debug: FluidDebug,
}
fn load_texture(
    device: &Device,
    bytes: &[u8],
    label: &str,
    format: wgpu::TextureFormat,
) -> Result<Texture, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .into_rgba8();
    Texture::from_rgba8(
        device,
        image.width(),
        image.height(),
        image.as_raw(),
        TextureOptions {
            label: label.into(),
            format,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())
}
fn depth_texture(device: &Device, width: u32, height: u32) -> BelfastResult<Texture> {
    Texture::create_2d(
        device,
        width,
        height,
        TextureOptions {
            label: "BlocksDepth".into(),
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..Default::default()
        },
    )
}
impl Renderer {
    pub fn new(
        device: &Device,
        particles: &Particles,
        fluid: &FluidSimulation,
    ) -> Result<Self, String> {
        let cube = Geom::cube(0.05);
        let vertices: Vec<[f32; 6]> = cube
            .positions
            .chunks_exact(3)
            .zip(cube.normals.chunks_exact(3))
            .map(|(p, n)| [p[0], p[1], p[2], n[0], n[1], n[2]])
            .collect();
        let mut mesh = Mesh::new(vertices.len() as u32)
            .map_err(|e| e.to_string())?
            .add_vertex_buffer(VertexBufferBinding {
                buffer: Buffer::from_data(device, &vertices, BufferUsage::vertex(), "BlocksCube"),
                array_stride: 24,
                attributes: vec![
                    VertexAttributeDescriptor {
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                    },
                    VertexAttributeDescriptor {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 12,
                    },
                ],
                slot: None,
                step_mode: None,
            })
            .map_err(|e| e.to_string())?;
        mesh.set_index_buffer(
            Buffer::from_data(device, &cube.indices, BufferUsage::index(), "BlocksIndices"),
            cube.indices.len() as u32,
            MeshIndexFormat::Uint32,
        )
        .map_err(|e| e.to_string())?;
        let buffer_entry = |ty, visibility| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let globals_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BlocksGlobals"),
                    entries: &[buffer_entry(
                        wgpu::BufferBindingType::Uniform,
                        wgpu::ShaderStages::VERTEX_FRAGMENT,
                    )],
                });
        let particle_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BlocksInstances"),
                    entries: &[buffer_entry(
                        wgpu::BufferBindingType::Storage { read_only: true },
                        wgpu::ShaderStages::VERTEX,
                    )],
                });
        let texture_entry = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(ty),
            count: None,
        };
        let material_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BlocksMaterial"),
                    entries: &[
                        texture_entry(0, wgpu::TextureSampleType::Float { filterable: true }),
                        sampler_entry(1, wgpu::SamplerBindingType::Filtering),
                        texture_entry(2, wgpu::TextureSampleType::Depth),
                        sampler_entry(3, wgpu::SamplerBindingType::Comparison),
                    ],
                });
        let layout = device
            .gpu()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("BlocksLayout"),
                bind_group_layouts: &[
                    Some(&globals_layout),
                    Some(&particle_layout),
                    Some(&material_layout),
                ],
                immediate_size: 0,
            });
        let depth_state = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        };
        let mut options = DrawOptions::new("Blocks", device.format());
        options.layout = Some(&layout);
        options.depth_stencil = Some(depth_state.clone());
        options.primitive.cull_mode = Some(wgpu::Face::Back);
        let source = shader_source();
        let draw = Draw::new(device, &source, &mesh, options);
        let depth_layout = device
            .gpu()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("BlocksShadowLayout"),
                bind_group_layouts: &[Some(&globals_layout), Some(&particle_layout)],
                immediate_size: 0,
            });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("BlocksShadowShader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let shadow_pipeline =
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("BlocksShadow"),
                    layout: Some(&depth_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_shadow"),
                        compilation_options: Default::default(),
                        buffers: &mesh.vertex_layouts(),
                    },
                    fragment: None,
                    primitive: wgpu::PrimitiveState {
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        bias: wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 2.0,
                            clamp: 0.0,
                        },
                        ..depth_state
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let globals = Buffer::create(device, 144, BufferUsage::uniform(), "BlocksGlobals");
        let globals_group = BindGroup::create(
            device,
            &globals_layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.gpu().as_entire_binding(),
            }],
            "BlocksGlobalsBinding",
        );
        let particle_groups = std::array::from_fn(|i| {
            BindGroup::create(
                device,
                &particle_layout,
                &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles.buffers[i].gpu().as_entire_binding(),
                }],
                "BlocksInstancesBinding",
            )
        });
        let shadow = Texture::create_2d(
            device,
            SHADOW_SIZE,
            SHADOW_SIZE,
            TextureOptions::shadow_map("BlocksShadow"),
        )
        .map_err(|e| e.to_string())?;
        let palette = load_texture(
            device,
            include_bytes!("../assets/colorMap.jpg"),
            "BlocksPalette",
            wgpu::TextureFormat::Rgba8Unorm,
        )?;
        let material_group = BindGroup::create(
            device,
            &material_layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(palette.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(palette.sampler()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(shadow.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(shadow.sampler()),
                },
            ],
            "BlocksMaterialBinding",
        );
        let background_mesh = Mesh::new(3).map_err(|e| e.to_string())?;
        let mut bg_options = DrawOptions::new("BlocksBackground", device.format());
        bg_options.depth_stencil = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let background = Draw::new(device, BACKGROUND, &background_mesh, bg_options);
        let bg = load_texture(
            device,
            include_bytes!("../assets/bg.jpg"),
            "BlocksBackground",
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )?;
        let background_group = BindGroup::create(
            device,
            &background.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(bg.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(bg.sampler()),
                },
            ],
            "BlocksBackgroundBinding",
        );
        Ok(Self {
            mesh,
            draw,
            shadow_pipeline,
            globals,
            globals_group,
            particle_groups,
            material_group,
            shadow,
            depth: depth_texture(device, 1, 1).map_err(|e| e.to_string())?,
            background,
            background_mesh,
            background_group,
            fluid_debug: FluidDebug::new(device, fluid)?,
        })
    }
    pub fn resize(
        &mut self,
        device: &Device,
        width: u32,
        height: u32,
        pixel_ratio: f32,
    ) -> BelfastResult<()> {
        if self.depth.width() != width || self.depth.height() != height {
            self.depth = depth_texture(device, width, height)?;
        }
        self.fluid_debug.resize(device, width, height, pixel_ratio);
        Ok(())
    }
    pub fn render(
        &self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        particles: &Particles,
        camera: &PerspectiveCamera,
        size: f32,
        shadow_strength: f32,
        show_debug_maps: bool,
    ) {
        let light = Mat4::orthographic_rh(-9.0, 9.0, -7.0, 7.0, 0.1, 24.0)
            * Mat4::look_at_rh(Vec3::new(0.0, 7.0, 5.0), Vec3::ZERO, Vec3::Y);
        let mut uniforms = camera.view_projection_matrix().to_vec();
        uniforms.extend(light.to_cols_array());
        uniforms.extend([size, shadow_strength, SHADOW_SIZE as f32, 0.0]);
        self.globals.write(device, &uniforms, 0);
        {
            let mut pass = self
                .shadow
                .begin_depth_pass(encoder, wgpu::LoadOp::Clear(1.0));
            pass.set_pipeline(&self.shadow_pipeline);
            self.globals_group.bind(&mut pass, 0);
            self.particle_groups[particles.current].bind(&mut pass, 1);
            self.mesh.bind(&mut pass);
            pass.draw_indexed(0..self.mesh.index_count(), 0, 0..COUNT);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("BlocksColor"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth.view(),
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
        self.background_group.bind(&mut pass, 0);
        self.background.draw(&mut pass, &self.background_mesh, 1);
        self.globals_group.bind(&mut pass, 0);
        self.particle_groups[particles.current].bind(&mut pass, 1);
        self.material_group.bind(&mut pass, 2);
        self.draw.draw(&mut pass, &self.mesh, COUNT);
        drop(pass);
        if show_debug_maps {
            self.fluid_debug.render(encoder, view);
        }
    }
}
