use crate::{Device, Mesh};

pub struct DrawOptions<'a> {
    pub label: &'a str,
    pub layout: Option<&'a wgpu::PipelineLayout>,
    pub primitive: wgpu::PrimitiveState,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub targets: Vec<Option<wgpu::ColorTargetState>>,
    pub multisample: wgpu::MultisampleState,
}

impl<'a> DrawOptions<'a> {
    pub fn new(label: &'a str, format: wgpu::TextureFormat) -> Self {
        Self {
            label,
            layout: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            targets: vec![Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            multisample: wgpu::MultisampleState::default(),
        }
    }
}

pub struct Draw {
    pipeline: wgpu::RenderPipeline,
}

impl Draw {
    pub fn new(device: &Device, shader_code: &str, mesh: &Mesh, options: DrawOptions<'_>) -> Self {
        let module = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("{}Shader", options.label)),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let layouts: Vec<_> = mesh.vertex_layouts().into_iter().flatten().collect();
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("{}Pipeline", options.label)),
                layout: options.layout,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &layouts,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &options.targets,
                }),
                primitive: options.primitive,
                depth_stencil: options.depth_stencil,
                multisample: options.multisample,
                multiview_mask: None,
                cache: None,
            });

        Self { pipeline }
    }

    pub fn get_bind_group_layout(&self, index: u32) -> wgpu::BindGroupLayout {
        self.pipeline.get_bind_group_layout(index)
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a Mesh,
        instance_count: u32,
    ) {
        pass.set_pipeline(&self.pipeline);
        mesh.bind(pass);
        if mesh.has_index_buffer() {
            pass.draw_indexed(0..mesh.index_count(), 0, 0..instance_count);
        } else {
            pass.draw(0..mesh.vertex_count(), 0..instance_count);
        }
    }
}
