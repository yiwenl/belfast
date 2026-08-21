mod common;

use belfast::{
    Buffer, BufferUsage, Draw, DrawOptions, Mesh, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext, ExampleRunOptions};

struct HdrDisplay {
    mesh: Mesh,
    draw: Draw,
}

impl Example for HdrDisplay {
    fn new(context: &ExampleContext) -> Self {
        eprintln!(
            "hdr_display: linear 0–8 ramp, red tick at SDR white (1.0); hdr={} {:?}",
            context.hdr, context.color_space
        );
        let positions = [-1.0_f32, -1.0, 3.0, -1.0, -1.0, 3.0];
        let position_buffer = Buffer::from_data(
            &context.device,
            &positions,
            BufferUsage::vertex(),
            "HdrDisplayPositions",
        );
        let mesh = Mesh::new(3)
            .expect("hdr display mesh")
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
            include_str!("shaders/hdr_display.wgsl"),
            &mesh,
            DrawOptions::new("HdrDisplay", context.format),
        );
        Self { mesh, draw }
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("HdrDisplayEncoder"),
                });
        {
            let mut pass = common::begin_render_pass(
                &mut encoder,
                surface_view,
                wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
            );
            self.draw.draw(&mut pass, &self.mesh, 1);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn main() {
    common::run_with::<HdrDisplay>(
        "Belfast Rust - HDR Display",
        ExampleRunOptions { hdr: true },
    );
}
