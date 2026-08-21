mod common;

use belfast::{
    BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Draw, DrawOptions, Mesh, UniformBlock,
    UniformFieldType, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext, Instant};

struct ComputeTriangle {
    mesh: Mesh,
    draw: Draw,
    compute: Compute,
    bind_group: BindGroup,
    params_buffer: Buffer,
    params: UniformBlock,
    started_at: Instant,
}

impl Example for ComputeTriangle {
    fn new(context: &ExampleContext) -> Self {
        let rest_positions = [
            0.0_f32, 0.7, 0.0, 1.0, -0.65, -0.55, 0.0, 1.0, 0.65, -0.55, 0.0, 1.0,
        ];
        let colors = [1.0_f32, 0.35, 0.2, 0.2, 0.85, 0.45, 0.25, 0.45, 1.0];
        let position_buffer = Buffer::from_data(
            &context.device,
            &rest_positions,
            BufferUsage::vertex_storage(),
            "ComputeTrianglePositions",
        );
        let color_buffer = Buffer::from_data(
            &context.device,
            &colors,
            BufferUsage::vertex(),
            "ComputeTriangleColors",
        );
        let mesh = Mesh::new(3)
            .expect("compute triangle mesh")
            .add_vertex_buffer(VertexBufferBinding {
                buffer: position_buffer.clone(),
                array_stride: 16,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
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
            include_str!("shaders/compute_triangle_draw.wgsl"),
            &mesh,
            DrawOptions::new("ComputeTriangleDraw", context.format),
        );

        let mut params =
            UniformBlock::create([("time", UniformFieldType::F32)]).expect("compute params schema");
        let params_buffer = Buffer::create(
            &context.device,
            params.byte_size() as u64,
            BufferUsage::uniform(),
            "ComputeTriangleParams",
        );
        params.set_f32("time", 0.0).expect("init time");
        params_buffer.write(&context.device, params.bytes(), 0);

        let compute = Compute::new(
            &context.device,
            include_str!("shaders/compute_triangle.wgsl"),
            ComputeOptions::new("ComputeTriangle"),
        );
        let bind_group = BindGroup::create(
            &context.device,
            &compute.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: position_buffer.gpu().as_entire_binding(),
                },
            ],
            "ComputeTriangleBindGroup",
        );

        Self {
            mesh,
            draw,
            compute,
            bind_group,
            params_buffer,
            params,
            started_at: Instant::now(),
        }
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        self.params
            .set_f32("time", self.started_at.elapsed().as_secs_f32())
            .expect("write time");
        self.params_buffer
            .write(&context.device, self.params.bytes(), 0);

        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ComputeTriangleEncoder"),
                });
        self.compute.run(
            &mut encoder,
            Some(&self.bind_group),
            [1, 1, 1],
            Some("ComputeTriangleDispatch"),
        );
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
    common::run::<ComputeTriangle>("Belfast Rust - Compute Triangle");
}
