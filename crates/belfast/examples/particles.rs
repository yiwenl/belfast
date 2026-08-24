mod common;

use belfast::{
    AxisHelper, AxisHelperOptions, BindGroup, Buffer, BufferUsage, CameraHelper,
    CameraHelperOptions, Compute, ComputeOptions, Device, Draw, DrawOptions, Geom, Mesh,
    MeshIndexFormat, OrbitalControl, OrbitalControlOptions, OrthographicCamera, PerspectiveCamera,
    Texture, TextureOptions, UniformBlock, UniformFieldType, VertexAttributeDescriptor,
    VertexBufferBinding, WGSL_SHADOW_PCF_3X3,
};
use common::{Example, ExampleContext, InputEvent};
use rand::Rng;

const PARTICLE_COUNT: u32 = 1_000_000;
const SPAWN_RADIUS: f32 = 1.5;
const MAX_RADIUS: f32 = 2.5;
const PARTICLE_STRIDE: u64 = 48;
const WORKGROUP_SIZE: u32 = 256;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const SHADOW_MAP_SIZE: u32 = 1024;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    pos_size: [f32; 4],
    velocity: [f32; 4],
    color: [f32; 4],
}

struct Experiment {
    mesh: Mesh,
    draw: Draw,
    axis: AxisHelper,
    light_frustum: CameraHelper,
    camera: PerspectiveCamera,
    control: OrbitalControl,
    uniform_block: UniformBlock,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    shadow_bind_group: BindGroup,
    compute: Compute,
    sim_params: UniformBlock,
    sim_buffer: Buffer,
    sim_bind_group: BindGroup,
    shadow_map: Texture,
    shadow_draw: Draw,
    light_bind_group: BindGroup,
    depth: Texture,
    time: f32,
}

impl Example for Experiment {
    fn new(context: &ExampleContext) -> Self {
        let geometry = Geom::plane(1.0, 1.0);
        let position_buffer = Buffer::from_data(
            &context.device,
            &geometry.positions,
            BufferUsage::vertex(),
            "ParticleQuadPositions",
        );
        let particles = spawn_particles();
        let instance_buffer = Buffer::from_data(
            &context.device,
            &particles,
            BufferUsage::vertex_storage(),
            "ParticleInstances",
        );
        let index_buffer = Buffer::from_data(
            &context.device,
            &geometry.indices,
            BufferUsage::index(),
            "ParticleQuadIndices",
        );
        let mut mesh = Mesh::new(4)
            .expect("particle quad mesh")
            .add_vertex_buffer(VertexBufferBinding {
                buffer: position_buffer,
                array_stride: 12,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                }],
                slot: Some(0),
                step_mode: None,
            })
            .expect("quad positions")
            .add_vertex_buffer(VertexBufferBinding {
                buffer: instance_buffer.clone(),
                array_stride: PARTICLE_STRIDE,
                attributes: vec![
                    VertexAttributeDescriptor {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                    },
                    VertexAttributeDescriptor {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 32,
                    },
                ],
                slot: Some(1),
                step_mode: Some(wgpu::VertexStepMode::Instance),
            })
            .expect("particle instances");
        mesh.set_index_buffer(
            index_buffer,
            geometry.indices.len() as u32,
            MeshIndexFormat::Uint32,
        )
        .expect("quad indices");

        let uniform_bind_group_layout =
            context
                .device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ExperimentUniformBindGroupLayout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let shadow_bind_group_layout =
            context
                .device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ExperimentShadowBindGroupLayout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
                    ],
                });
        let axis_pipeline_layout =
            context
                .device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ExperimentAxisPipelineLayout"),
                    bind_group_layouts: &[Some(&uniform_bind_group_layout)],
                    immediate_size: 0,
                });
        let draw_pipeline_layout =
            context
                .device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ExperimentDrawPipelineLayout"),
                    bind_group_layouts: &[
                        Some(&uniform_bind_group_layout),
                        Some(&shadow_bind_group_layout),
                    ],
                    immediate_size: 0,
                });
        let draw_shader = format!(
            "{}\n{}",
            WGSL_SHADOW_PCF_3X3,
            include_str!("shaders/particles.wgsl")
        );
        let mut draw_options = DrawOptions::new("Particles", context.format);
        draw_options.layout = Some(&draw_pipeline_layout);
        draw_options.depth_stencil = Some(depth_stencil_state());
        let draw = Draw::new(&context.device, &draw_shader, &mesh, draw_options);
        let axis = AxisHelper::new(
            &context.device,
            AxisHelperOptions {
                length: 100.0,
                depth_stencil: Some(depth_stencil_state()),
                ..AxisHelperOptions::new(context.format, &axis_pipeline_layout)
            },
        )
        .expect("axis helper");

        let light_r = 6.5;
        let mut light_camera =
            OrthographicCamera::new(-light_r, light_r, -light_r, light_r, 1.0, 16.0);
        light_camera.look_at([0.0, 8.0, 0.5], [0.0, 0.0, 0.0]);
        let light_frustum = CameraHelper::new(
            &context.device,
            &light_camera.view_projection_matrix(),
            CameraHelperOptions {
                color: [1.0, 0.85, 0.2],
                depth_stencil: Some(depth_stencil_state()),
                ..CameraHelperOptions::new(context.format, &axis_pipeline_layout)
            },
        )
        .expect("light camera helper");
        let light_view = light_camera.view_matrix();
        let (light_right, light_up) = view_right_up(&light_view);

        let mut uniform_block = UniformBlock::create([
            ("view_proj", UniformFieldType::Mat4x4F),
            ("camera_right", UniformFieldType::Vec4F),
            ("camera_up", UniformFieldType::Vec4F),
            ("light_view_proj", UniformFieldType::Mat4x4F),
            ("shadow_params", UniformFieldType::Vec4F),
        ])
        .expect("uniform block");
        uniform_block
            .set_f32_slice("light_view_proj", &light_camera.view_projection_matrix())
            .expect("light_view_proj");
        uniform_block
            .set_f32_slice("shadow_params", &[0.8, SHADOW_MAP_SIZE as f32, 0.001, 0.0])
            .expect("shadow_params");
        let uniform_buffer = Buffer::create(
            &context.device,
            uniform_block.byte_size() as u64,
            BufferUsage::uniform(),
            "ExperimentUniformBuffer",
        );
        let uniform_bind_group = BindGroup::create(
            &context.device,
            &uniform_bind_group_layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.gpu().as_entire_binding(),
            }],
            "ExperimentUniformBindGroup",
        );

        let shadow_map = Texture::create_2d(
            &context.device,
            SHADOW_MAP_SIZE,
            SHADOW_MAP_SIZE,
            TextureOptions::shadow_map("ParticleShadowMap"),
        )
        .expect("shadow map");
        let shadow_bind_group = BindGroup::create(
            &context.device,
            &shadow_bind_group_layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(shadow_map.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(shadow_map.sampler()),
                },
            ],
            "ParticleShadowBindGroup",
        );

        let mut light_uniform = UniformBlock::create([
            ("view_proj", UniformFieldType::Mat4x4F),
            ("camera_right", UniformFieldType::Vec4F),
            ("camera_up", UniformFieldType::Vec4F),
        ])
        .expect("light uniform");
        light_uniform
            .set_f32_slice("view_proj", &light_camera.view_projection_matrix())
            .expect("light view_proj");
        light_uniform
            .set_f32_slice("camera_right", &light_right)
            .expect("light right");
        light_uniform
            .set_f32_slice("camera_up", &light_up)
            .expect("light up");
        let light_buffer = Buffer::create(
            &context.device,
            light_uniform.byte_size() as u64,
            BufferUsage::uniform(),
            "ParticleLightUniform",
        );
        light_buffer.write(&context.device, light_uniform.bytes(), 0);
        let light_bind_group = BindGroup::create(
            &context.device,
            &uniform_bind_group_layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.gpu().as_entire_binding(),
            }],
            "ParticleLightBindGroup",
        );
        let mut shadow_draw_options = DrawOptions::depth_only("ParticleShadow");
        shadow_draw_options.layout = Some(&axis_pipeline_layout);
        let shadow_draw = Draw::new(
            &context.device,
            include_str!("shaders/particles_shadow_depth.wgsl"),
            &mesh,
            shadow_draw_options,
        );

        let time = rand::rng().random_range(0.0..1000.0);
        let mut sim_params = UniformBlock::create([
            ("time", UniformFieldType::F32),
            ("dt", UniformFieldType::F32),
            ("max_radius", UniformFieldType::F32),
            ("count", UniformFieldType::U32),
            ("noise_scale", UniformFieldType::F32),
            ("force_scale", UniformFieldType::F32),
            ("damping", UniformFieldType::F32),
            ("center_force", UniformFieldType::F32),
        ])
        .expect("sim params schema");
        sim_params.set_f32("time", time).expect("init time");
        sim_params.set_f32("dt", 0.016).expect("init dt");
        sim_params
            .set_f32("max_radius", MAX_RADIUS)
            .expect("init max_radius");
        sim_params
            .set_u32("count", PARTICLE_COUNT)
            .expect("init count");
        sim_params
            .set_f32("noise_scale", 0.6)
            .expect("init noise_scale");
        sim_params
            .set_f32("force_scale", 2.0)
            .expect("init force_scale");
        sim_params.set_f32("damping", 0.986).expect("init damping");
        sim_params
            .set_f32("center_force", 4.0)
            .expect("init center_force");
        let sim_buffer = Buffer::create(
            &context.device,
            sim_params.byte_size() as u64,
            BufferUsage::uniform(),
            "ParticleSimParams",
        );
        sim_buffer.write(&context.device, sim_params.bytes(), 0);

        let compute = Compute::new(
            &context.device,
            include_str!("shaders/particles_update.wgsl"),
            ComputeOptions::new("ParticleUpdate"),
        );
        let sim_bind_group = BindGroup::create(
            &context.device,
            &compute.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_buffer.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: instance_buffer.gpu().as_entire_binding(),
                },
            ],
            "ParticleSimBindGroup",
        );

        let mut camera = PerspectiveCamera::new(
            std::f32::consts::FRAC_PI_4,
            context.width as f32 / context.height as f32,
            0.1,
            100.0,
        );
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            radius: 20.0,
            ..Default::default()
        })
        .expect("valid orbital control");
        control.update(0.0, &mut camera);

        Self {
            mesh,
            draw,
            axis,
            light_frustum,
            camera,
            control,
            uniform_block,
            uniform_buffer,
            uniform_bind_group,
            shadow_bind_group,
            compute,
            sim_params,
            sim_buffer,
            sim_bind_group,
            shadow_map,
            shadow_draw,
            light_bind_group,
            depth: create_depth_texture(&context.device, context.width, context.height),
            time,
        }
    }

    fn input(&mut self, context: &ExampleContext, event: InputEvent) {
        match event {
            InputEvent::PointerDown {
                position,
                button,
                pan_modifier,
            } => {
                self.control.pointer_down(position, button, pan_modifier);
            }
            InputEvent::PointerMove { position } => {
                self.control
                    .pointer_move(position, [context.width as f32, context.height as f32]);
            }
            InputEvent::PointerUp { button } => self.control.pointer_up(button),
            InputEvent::Scroll { delta } => self.control.scroll(delta),
        }
    }

    fn resize(&mut self, context: &ExampleContext) {
        self.camera
            .set_aspect(context.width as f32 / context.height as f32)
            .expect("positive surface aspect");
        self.depth = create_depth_texture(&context.device, context.width, context.height);
    }

    fn update(&mut self, context: &ExampleContext, delta_seconds: f32) {
        self.control.update(delta_seconds, &mut self.camera);
        let view = self.camera.view_matrix();
        self.uniform_block
            .set_f32_slice("view_proj", &self.camera.view_projection_matrix())
            .expect("write camera matrix");
        self.uniform_block
            .set_f32_slice("camera_right", &[view[0], view[4], view[8], 0.0])
            .expect("write camera right");
        self.uniform_block
            .set_f32_slice("camera_up", &[view[1], view[5], view[9], 0.0])
            .expect("write camera up");
        self.uniform_buffer
            .write(&context.device, self.uniform_block.bytes(), 0);

        self.time += delta_seconds;
        self.sim_params
            .set_f32("time", self.time)
            .expect("write time");
        self.sim_params
            .set_f32("dt", delta_seconds)
            .expect("write dt");
        self.sim_buffer
            .write(&context.device, self.sim_params.bytes(), 0);
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ExperimentEncoder"),
                });
        let workgroups = PARTICLE_COUNT.div_ceil(WORKGROUP_SIZE);
        self.compute.run(
            &mut encoder,
            Some(&self.sim_bind_group),
            [workgroups, 1, 1],
            Some("ParticleUpdate"),
        );
        {
            let mut pass = self
                .shadow_map
                .begin_depth_pass(&mut encoder, wgpu::LoadOp::Clear(1.0));
            self.light_bind_group.bind(&mut pass, 0);
            self.shadow_draw.draw(&mut pass, &self.mesh, PARTICLE_COUNT);
        }
        {
            let mut pass = common::begin_render_pass_with_depth(
                &mut encoder,
                surface_view,
                self.depth.view(),
                wgpu::Color {
                    r: 0.08,
                    g: 0.076,
                    b: 0.068,
                    a: 1.0,
                },
            );

            self.uniform_bind_group.bind(&mut pass, 0);
            self.shadow_bind_group.bind(&mut pass, 1);
            self.draw.draw(&mut pass, &self.mesh, PARTICLE_COUNT);
            self.axis.draw(&mut pass, &self.uniform_bind_group);
            self.light_frustum.draw(&mut pass, &self.uniform_bind_group);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn create_depth_texture(device: &Device, width: u32, height: u32) -> Texture {
    Texture::create_2d(
        device,
        width.max(1),
        height.max(1),
        TextureOptions {
            label: "ParticleDepth".into(),
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..Default::default()
        },
    )
    .expect("particle depth texture")
}

fn view_right_up(view: &[f32; 16]) -> ([f32; 4], [f32; 4]) {
    (
        [view[0], view[4], view[8], 0.0],
        [view[1], view[5], view[9], 0.0],
    )
}

fn depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn spawn_particles() -> Vec<Particle> {
    let mut rng = rand::rng();
    (0..PARTICLE_COUNT)
        .map(|_| {
            let theta = rng.random::<f32>() * std::f32::consts::TAU;
            let phi = (2.0 * rng.random::<f32>() - 1.0).acos();
            let r = rng.random::<f32>().sqrt() * SPAWN_RADIUS;
            let sin_phi = phi.sin();
            let grey = rng.random_range(0.8..1.0);
            Particle {
                pos_size: [
                    r * sin_phi * theta.cos(),
                    r * sin_phi * theta.sin(),
                    r * phi.cos(),
                    rng.random_range(0.02..0.04),
                ],
                velocity: [
                    rng.random_range(-0.2..0.2),
                    rng.random_range(-0.2..0.2),
                    rng.random_range(-0.2..0.2),
                    rng.random::<f32>(),
                ],
                color: [grey, grey, grey, rng.random::<f32>()],
            }
        })
        .collect()
}

fn main() {
    common::run::<Experiment>("Belfast Rust - Particles");
}
