mod common;

use belfast::{
    AxisHelper, AxisHelperOptions, BindGroup, Buffer, BufferUsage, Draw, DrawOptions, Mesh,
    OrbitalControl, OrbitalControlOptions, PerspectiveCamera, UniformBlock, UniformFieldType,
    VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext, InputEvent};

const SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(position, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

struct Experiment {
    mesh: Mesh,
    draw: Draw,
    axis: AxisHelper,
    camera: PerspectiveCamera,
    control: OrbitalControl,
    camera_uniform: UniformBlock,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
}

impl Example for Experiment {
    fn new(context: &ExampleContext) -> Self {
        let positions = [0.0_f32, 0.7, 0.0, -0.65, -0.55, 0.0, 0.65, -0.55, 0.0];
        let colors = [1.0_f32, 0.25, 0.2, 0.15, 0.9, 0.4, 0.2, 0.45, 1.0];
        let position_buffer = Buffer::from_data(
            &context.device,
            &positions,
            BufferUsage::vertex(),
            "ExperimentPositions",
        );
        let color_buffer = Buffer::from_data(
            &context.device,
            &colors,
            BufferUsage::vertex(),
            "ExperimentColors",
        );
        let mesh = Mesh::new(3)
            .expect("experiment mesh")
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
            .expect("position buffer")
            .add_vertex_buffer(VertexBufferBinding {
                buffer: color_buffer,
                array_stride: 12,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                }],
                slot: Some(1),
                step_mode: None,
            })
            .expect("color buffer");

        let camera_bind_group_layout =
            context
                .device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ExperimentCameraLayout"),
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
                    bind_group_layouts: &[Some(&camera_bind_group_layout)],
                    immediate_size: 0,
                });
        let mut draw_options = DrawOptions::new("Experiment", context.format);
        draw_options.layout = Some(&pipeline_layout);
        let draw = Draw::new(&context.device, SHADER, &mesh, draw_options);
        let axis = AxisHelper::new(
            &context.device,
            AxisHelperOptions {
                length: 1.5,
                ..AxisHelperOptions::new(context.format, &pipeline_layout)
            },
        )
        .expect("axis helper");

        let camera_uniform = UniformBlock::create([("view_proj", UniformFieldType::Mat4x4F)])
            .expect("camera uniform schema");
        let camera_buffer = Buffer::create(
            &context.device,
            camera_uniform.byte_size() as u64,
            BufferUsage::uniform(),
            "ExperimentCameraBuffer",
        );
        let camera_bind_group = BindGroup::create(
            &context.device,
            &camera_bind_group_layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.gpu().as_entire_binding(),
            }],
            "ExperimentCameraBindGroup",
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
            camera_uniform,
            camera_buffer,
            camera_bind_group,
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
        self.camera_uniform
            .set_f32_slice("view_proj", &self.camera.view_projection_matrix())
            .expect("write camera matrix");
        self.camera_buffer
            .write(&context.device, self.camera_uniform.bytes(), 0);
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
                    r: 0.025,
                    g: 0.025,
                    b: 0.045,
                    a: 1.0,
                },
            );
            self.camera_bind_group.bind(&mut pass, 0);
            self.draw.draw(&mut pass, &self.mesh, 1);
            self.axis.draw(&mut pass, &self.camera_bind_group);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn main() {
    common::run::<Experiment>("Belfast Rust - Experiment Template");
}
