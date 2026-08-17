mod common;

use belfast::{
    Buffer, BufferUsage, Draw, DrawOptions, Mesh, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext};

struct Triangle {
    mesh: Mesh,
    draw: Draw,
}

impl Example for Triangle {
    fn new(context: &ExampleContext) -> Self {
        let positions = [0.0_f32, 0.6, -0.6, -0.5, 0.6, -0.5];
        let position_buffer = Buffer::from_data(
            &context.device,
            &positions,
            BufferUsage::vertex(),
            "TrianglePositions",
        );
        let mesh = Mesh::new(3)
            .expect("triangle mesh")
            .add_vertex_buffer(VertexBufferBinding {
                buffer: position_buffer,
                array_stride: 8,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                }],
                slot: Some(0),
                step_mode: None,
            })
            .expect("position buffer");
        let draw = Draw::new(
            &context.device,
            include_str!("shaders/triangle.wgsl"),
            &mesh,
            DrawOptions::new("Triangle", context.format),
        );
        Self { mesh, draw }
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TriangleEncoder"),
                });
        {
            let mut pass = common::begin_render_pass(
                &mut encoder,
                surface_view,
                wgpu::Color {
                    r: 0.02,
                    g: 0.025,
                    b: 0.04,
                    a: 1.0,
                },
            );
            self.draw.draw(&mut pass, &self.mesh, 1);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn main() {
    common::run::<Triangle>("Belfast Rust - Triangle");
}
