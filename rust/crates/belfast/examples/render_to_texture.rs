mod common;

use belfast::{
    BindGroup, Buffer, BufferUsage, Draw, DrawOptions, Mesh, RenderPassOptions, RenderTarget,
    RenderTargetOptions, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext};

struct RenderToTexture {
    source_mesh: Mesh,
    source_draw: Draw,
    present_mesh: Mesh,
    present_draw: Draw,
    present_layout: wgpu::BindGroupLayout,
    present_bind_group: BindGroup,
    target: RenderTarget,
}

impl RenderToTexture {
    fn create_present_bind_group(
        context: &ExampleContext,
        layout: &wgpu::BindGroupLayout,
        target: &RenderTarget,
    ) -> BindGroup {
        BindGroup::create(
            &context.device,
            layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(target.color_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(target.sampler()),
                },
            ],
            "RenderTargetPresentBindGroup",
        )
    }
}

impl Example for RenderToTexture {
    fn new(context: &ExampleContext) -> Self {
        let positions = [0.0_f32, 0.65, -0.65, -0.55, 0.65, -0.55];
        let colors = [1.0_f32, 0.2, 0.25, 0.15, 0.95, 0.5, 0.2, 0.45, 1.0];
        let position_buffer = Buffer::from_data(
            &context.device,
            &positions,
            BufferUsage::vertex(),
            "RenderTargetPositions",
        );
        let color_buffer = Buffer::from_data(
            &context.device,
            &colors,
            BufferUsage::vertex(),
            "RenderTargetColors",
        );
        let source_mesh = Mesh::new(3)
            .expect("source mesh")
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
        let source_draw = Draw::new(
            &context.device,
            include_str!("shaders/render_target_source.wgsl"),
            &source_mesh,
            DrawOptions::new("RenderTargetSource", context.format),
        );

        let present_mesh = Mesh::new(3).expect("fullscreen triangle mesh");
        let present_draw = Draw::new(
            &context.device,
            include_str!("shaders/render_target_present.wgsl"),
            &present_mesh,
            DrawOptions::new("RenderTargetPresent", context.format),
        );
        let present_layout = present_draw.get_bind_group_layout(0);
        let target = RenderTarget::create(
            &context.device,
            RenderTargetOptions {
                width: context.width,
                height: context.height,
                label: "ExampleOffscreenTarget".to_string(),
                format: Some(context.format),
                ..Default::default()
            },
        );
        let present_bind_group = Self::create_present_bind_group(context, &present_layout, &target);

        Self {
            source_mesh,
            source_draw,
            present_mesh,
            present_draw,
            present_layout,
            present_bind_group,
            target,
        }
    }

    fn resize(&mut self, context: &ExampleContext) {
        self.target.resize(context.width, context.height);
        self.present_bind_group =
            Self::create_present_bind_group(context, &self.present_layout, &self.target);
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("RenderToTextureEncoder"),
                });
        {
            let mut pass = self.target.begin_render_pass(
                &mut encoder,
                RenderPassOptions {
                    clear_color: wgpu::Color {
                        r: 0.03,
                        g: 0.035,
                        b: 0.07,
                        a: 1.0,
                    },
                    load_op: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.03,
                        g: 0.035,
                        b: 0.07,
                        a: 1.0,
                    }),
                    ..Default::default()
                },
            );
            self.source_draw.draw(&mut pass, &self.source_mesh, 1);
        }
        {
            let mut pass =
                common::begin_render_pass(&mut encoder, surface_view, wgpu::Color::BLACK);
            self.present_bind_group.bind(&mut pass, 0);
            self.present_draw.draw(&mut pass, &self.present_mesh, 1);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn main() {
    common::run::<RenderToTexture>("Belfast Rust - Render To Texture");
}
