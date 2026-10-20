use belfast::{Buffer, BufferUsage, Device, Texture};

/// Draw the actual PCF depth texture after scene rendering, without a CPU copy.
pub struct ShadowDebug {
    pipeline: wgpu::RenderPipeline,
    binding: wgpu::BindGroup,
}

impl ShadowDebug {
    pub fn new(device: &Device, map: &Texture) -> Self {
        let gpu = device.gpu();
        let options = Buffer::from_data(
            device,
            &[u32::from(device.format().is_srgb()) as f32, 0., 0., 0.],
            BufferUsage::uniform(),
            "ShadowDebugOptions",
        );
        let layout = gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ShadowDebugLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let binding = gpu.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ShadowDebugBinding"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(map.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: options.gpu().as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = gpu.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ShadowDebugPipelineLayout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = gpu.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ShadowDebugShader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow-debug.wgsl").into()),
        });
        let pipeline = gpu.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ShadowDebugPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: device.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, binding }
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: [u32; 2],
        rect: [f32; 4],
    ) {
        if rect[2] <= 0. || rect[3] <= 0. {
            return;
        }
        let [width, height] = viewport.map(|v| v as f32);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ShadowDebugOverlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(
            rect[0] * width,
            rect[1] * height,
            rect[2] * width,
            rect[3] * height,
            0.,
            1.,
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.draw(0..3, 0..1);
    }
}
