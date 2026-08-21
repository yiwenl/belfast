mod common;

use belfast::{
    BindGroup, Buffer, BufferUsage, Draw, DrawOptions, Geom, Mesh, MeshIndexFormat, Texture,
    TextureOptions, VertexAttributeDescriptor, VertexBufferBinding,
};
use common::{Example, ExampleContext};

struct TextureExample {
    _texture: Texture,
    mesh: Mesh,
    draw: Draw,
    bind_group: BindGroup,
}

impl Example for TextureExample {
    fn new(context: &ExampleContext) -> Self {
        let geometry = Geom::plane(1.5, 1.5);
        let position_buffer = Buffer::from_data(
            &context.device,
            &geometry.positions,
            BufferUsage::vertex(),
            "TexturePlanePositions",
        );
        let uv_buffer = Buffer::from_data(
            &context.device,
            &geometry.uvs,
            BufferUsage::vertex(),
            "TexturePlaneUvs",
        );
        let index_buffer = Buffer::from_data(
            &context.device,
            &geometry.indices,
            BufferUsage::index(),
            "TexturePlaneIndices",
        );
        let mut mesh = Mesh::new(4)
            .expect("texture plane mesh")
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
                buffer: uv_buffer,
                array_stride: 8,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                }],
                slot: Some(1),
                step_mode: None,
            })
            .expect("uv buffer");
        mesh.set_index_buffer(
            index_buffer,
            geometry.indices.len() as u32,
            MeshIndexFormat::Uint32,
        )
        .expect("index buffer");

        let pixels = checkerboard_rgba8(64, 64);
        let texture = Texture::from_rgba8(
            &context.device,
            64,
            64,
            &pixels,
            TextureOptions {
                label: "CheckerboardTexture".to_string(),
                ..Default::default()
            },
        )
        .expect("checkerboard texture");
        let draw = Draw::new(
            &context.device,
            include_str!("shaders/texture.wgsl"),
            &mesh,
            DrawOptions::new("Texture", context.format),
        );
        let layout = draw.get_bind_group_layout(0);
        let bind_group = BindGroup::create(
            &context.device,
            &layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(texture.sampler()),
                },
            ],
            "TextureBindGroup",
        );

        Self {
            _texture: texture,
            mesh,
            draw,
            bind_group,
        }
    }

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TextureEncoder"),
                });
        {
            let mut pass = common::begin_render_pass(
                &mut encoder,
                surface_view,
                wgpu::Color {
                    r: 0.03,
                    g: 0.03,
                    b: 0.035,
                    a: 1.0,
                },
            );
            self.bind_group.bind(&mut pass, 0);
            self.draw.draw(&mut pass, &self.mesh, 1);
        }
        context.device.queue().submit([encoder.finish()]);
    }
}

fn checkerboard_rgba8(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let bright = ((x / 8) + (y / 8)) % 2 == 0;
            let color = if bright {
                [240, 205, 70, 255]
            } else {
                [28, 54, 92, 255]
            };
            pixels.extend_from_slice(&color);
        }
    }
    pixels
}

fn main() {
    common::run::<TextureExample>("Belfast Rust - Texture");
}
