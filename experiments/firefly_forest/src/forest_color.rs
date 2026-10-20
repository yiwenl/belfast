use belfast::Device;

/// Static forest color; Scene invalidates it together with forest occlusion.
pub struct ForestColor {
    pub view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

impl ForestColor {
    pub fn new(device: &Device) -> Self {
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ForestColorLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ForestColorCopyShader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("forest-color.wgsl").into()),
            });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ForestColorCopyLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ForestColorCopyPipeline"),
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
        let (view, bind_group) = Self::target(device, &layout, [1, 1]);
        Self {
            view,
            bind_group,
            layout,
            pipeline,
        }
    }

    fn target(
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        size: [u32; 2],
    ) -> (wgpu::TextureView, wgpu::BindGroup) {
        let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("ForestColorCache"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: device.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ForestColorCopyBindGroup"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        (view, bind_group)
    }

    pub fn resize(&mut self, device: &Device, size: [u32; 2]) {
        (self.view, self.bind_group) = Self::target(device, &self.layout, size);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
