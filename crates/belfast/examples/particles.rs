mod common;

use belfast::{
    AxisHelper, AxisHelperOptions, BindGroup, Buffer, BufferUsage, Draw, DrawOptions, Geom, Mesh,
    MeshIndexFormat, OrbitalControl, OrbitalControlOptions, PerspectiveCamera, UniformBlock,
    UniformFieldType, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext, InputEvent};

const PARTICLE_COUNT: u32 = 10_000;
const SPAWN_RADIUS: f32 = 1.5;
const PARTICLE_STRIDE: u64 = 28;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: [f32; 3],
    color: [f32; 3],
    size: f32,
}

struct Experiment {
    mesh: Mesh,
    draw: Draw,
    axis: AxisHelper,
    camera: PerspectiveCamera,
    control: OrbitalControl,
    uniform_block: UniformBlock,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
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
        debug_assert_eq!(particles.len(), PARTICLE_COUNT as usize);
        debug_assert_eq!(std::mem::size_of::<Particle>() as u64, PARTICLE_STRIDE);
        let instance_buffer = Buffer::from_data(
            &context.device,
            &particles,
            BufferUsage::vertex(),
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
                buffer: instance_buffer,
                array_stride: PARTICLE_STRIDE,
                attributes: vec![
                    VertexAttributeDescriptor {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                    },
                    VertexAttributeDescriptor {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 12,
                    },
                    VertexAttributeDescriptor {
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32,
                        offset: 24,
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
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let pipeline_layout =
            context
                .device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ExperimentPipelineLayout"),
                    bind_group_layouts: &[Some(&uniform_bind_group_layout)],
                    immediate_size: 0,
                });
        let mut draw_options = DrawOptions::new("Particles", context.format);
        draw_options.layout = Some(&pipeline_layout);
        let draw = Draw::new(
            &context.device,
            include_str!("shaders/particles.wgsl"),
            &mesh,
            draw_options,
        );
        let axis = AxisHelper::new(
            &context.device,
            AxisHelperOptions {
                length: 1.5,
                ..AxisHelperOptions::new(context.format, &pipeline_layout)
            },
        )
        .expect("axis helper");

        let uniform_block = UniformBlock::create([
            ("view_proj", UniformFieldType::Mat4x4F),
            ("camera_right", UniformFieldType::Vec4F),
            ("camera_up", UniformFieldType::Vec4F),
        ])
        .expect("uniform block");
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

        let mut camera = PerspectiveCamera::new(
            std::f32::consts::FRAC_PI_4,
            context.width as f32 / context.height as f32,
            0.1,
            100.0,
        );
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            radius: 4.0,
            ..Default::default()
        })
        .expect("valid orbital control");
        control.update(0.0, &mut camera);

        Self {
            mesh,
            draw,
            axis,
            camera,
            control,
            uniform_block,
            uniform_buffer,
            uniform_bind_group,
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
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ExperimentEncoder"),
                });
        {
            let mut pass = common::begin_render_pass(
                &mut encoder,
                surface_view,
                wgpu::Color {
                    r: 0.01,
                    g: 0.0095,
                    b: 0.0085,
                    a: 1.0,
                },
            );

            self.uniform_bind_group.bind(&mut pass, 0);
            self.draw.draw(&mut pass, &self.mesh, PARTICLE_COUNT);
            self.axis.draw(&mut pass, &self.uniform_bind_group);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn spawn_particles() -> Vec<Particle> {
    let mut rng = 0x6d2b79f5_u32;
    (0..PARTICLE_COUNT)
        .map(|_| {
            let theta = rand_f32(&mut rng) * std::f32::consts::TAU;
            let phi = (2.0 * rand_f32(&mut rng) - 1.0).acos();
            let r = rand_f32(&mut rng).cbrt() * SPAWN_RADIUS;
            let sin_phi = phi.sin();
            let hue = rand_f32(&mut rng) * std::f32::consts::TAU;
            Particle {
                position: [
                    r * sin_phi * theta.cos(),
                    r * sin_phi * theta.sin(),
                    r * phi.cos(),
                ],
                color: [
                    0.5 + 0.5 * hue.cos(),
                    0.5 + 0.5 * (hue + 2.094_395).cos(),
                    0.5 + 0.5 * (hue + 4.188_79).cos(),
                ],
                size: 0.02 + rand_f32(&mut rng) * 0.06,
            }
        })
        .collect()
}

fn rand_f32(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 8) as f32 / 16_777_216.0
}

fn main() {
    common::run::<Experiment>("Belfast Rust - Particles");
}
