use crate::{Device, Draw, DrawOptions, Mesh};

pub struct DepthDrawOptions<'a> {
    pub label: &'a str,
    pub layout: Option<&'a wgpu::PipelineLayout>,
    pub primitive: wgpu::PrimitiveState,
    pub depth_format: wgpu::TextureFormat,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> DepthDrawOptions<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            layout: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_format: wgpu::TextureFormat::Depth32Float,
            depth_stencil: None,
        }
    }
}

pub struct DepthDraw {
    draw: Draw,
}

impl DepthDraw {
    pub fn new(
        device: &Device,
        shader_code: &str,
        mesh: &Mesh,
        options: DepthDrawOptions<'_>,
    ) -> Self {
        let depth_stencil = options.depth_stencil.unwrap_or(wgpu::DepthStencilState {
            format: options.depth_format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let draw = Draw::new(
            device,
            shader_code,
            mesh,
            DrawOptions {
                label: options.label,
                layout: options.layout,
                primitive: options.primitive,
                depth_stencil: Some(depth_stencil),
                targets: vec![],
                multisample: wgpu::MultisampleState::default(),
            },
        );
        Self { draw }
    }

    pub fn get_bind_group_layout(&self, index: u32) -> wgpu::BindGroupLayout {
        self.draw.get_bind_group_layout(index)
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a Mesh,
        instance_count: u32,
    ) {
        self.draw.draw(pass, mesh, instance_count);
    }
}
