mod common;

use belfast::{
    BindGroup, Buffer, BufferUsage, Draw, DrawOptions, Mesh, PerspectiveCamera, UniformBlock,
    UniformFieldType, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext, Instant};

struct CameraUniform {
    mesh: Mesh,
    draw: Draw,
    bind_group: BindGroup,
    uniform_buffer: Buffer,
    uniform_block: UniformBlock,
    camera: PerspectiveCamera,
    started_at: Instant,
}

impl Example for CameraUniform {
    fn new(context: &ExampleContext) -> Self {
        let positions = [0.0_f32, 0.7, 0.0, -0.65, -0.55, 0.0, 0.65, -0.55, 0.0];
        let colors = [1.0_f32, 0.25, 0.2, 0.15, 0.9, 0.4, 0.2, 0.45, 1.0];
        let position_buffer = Buffer::from_data(
            &context.device,
            &positions,
            BufferUsage::vertex(),
            "CameraTrianglePositions",
        );
        let color_buffer = Buffer::from_data(
            &context.device,
            &colors,
            BufferUsage::vertex(),
            "CameraTriangleColors",
        );
        let mesh = Mesh::new(3)
            .expect("camera triangle mesh")
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

        let draw = Draw::new(
            &context.device,
            include_str!("shaders/camera_uniform.wgsl"),
            &mesh,
            DrawOptions::new("CameraUniform", context.format),
        );
        let layout = draw.get_bind_group_layout(0);
        let uniform_block = UniformBlock::create([("view_proj", UniformFieldType::Mat4x4F)])
            .expect("camera uniform schema");
        let uniform_buffer = Buffer::create(
            &context.device,
            uniform_block.byte_size() as u64,
            BufferUsage::uniform(),
            "CameraUniformBuffer",
        );
        let bind_group = BindGroup::create(
            &context.device,
            &layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.gpu().as_entire_binding(),
            }],
            "CameraUniformBindGroup",
        );
        let mut camera = PerspectiveCamera::new(
            std::f32::consts::FRAC_PI_4,
            context.width as f32 / context.height as f32,
            0.1,
            100.0,
        );
        camera.look_at([0.0, 0.0, 2.4], [0.0, 0.0, 0.0]);

        Self {
            mesh,
            draw,
            bind_group,
            uniform_buffer,
            uniform_block,
            camera,
            started_at: Instant::now(),
        }
    }

    fn resize(&mut self, context: &ExampleContext) {
        self.camera
            .set_aspect(context.width as f32 / context.height as f32)
            .expect("positive surface aspect");
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let time = self.started_at.elapsed().as_secs_f32();
        let eye = [time.sin() * 1.2, 0.45, time.cos() * 1.2 + 1.8];
        self.camera.look_at(eye, [0.0, 0.0, 0.0]);
        self.uniform_block
            .set_f32_slice("view_proj", &self.camera.view_projection_matrix())
            .expect("write camera matrix");
        self.uniform_buffer
            .write(&context.device, self.uniform_block.bytes(), 0);

        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("CameraUniformEncoder"),
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
            self.bind_group.bind(&mut pass, 0);
            self.draw.draw(&mut pass, &self.mesh, 1);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn main() {
    common::run::<CameraUniform>("Belfast Rust - Camera Uniform");
}
