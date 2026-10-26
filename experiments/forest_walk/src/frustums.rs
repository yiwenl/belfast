use crate::overview::FrustumInstance;
use belfast::{Buffer, BufferUsage, Device};

pub struct Frustums {
    pipeline: wgpu::RenderPipeline,
    current_pipeline: wgpu::RenderPipeline,
    recorded: Option<Buffer>,
    current: Buffer,
    count: u32,
}
impl Frustums {
    pub fn new(device: &Device, layout: &wgpu::PipelineLayout) -> Self {
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ForestCameraFrustums"),
                source: wgpu::ShaderSource::Wgsl(include_str!("frustums.wgsl").into()),
            });
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
        let make_pipeline = |entry_point, topology| {
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("InstancedCameraFrustums"),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some(entry_point),
                        compilation_options: Default::default(),
                        buffers: &[Some(wgpu::VertexBufferLayout {
                            array_stride: 80,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &attributes,
                        })],
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
                    primitive: wgpu::PrimitiveState {
                        topology,
                        ..Default::default()
                    },
                    // Camera markers remain legible through the dense canopy; current draws last.
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let pipeline = make_pipeline("vs_main", wgpu::PrimitiveTopology::LineList);
        let current_pipeline = make_pipeline("vs_current", wgpu::PrimitiveTopology::TriangleList);
        Self {
            pipeline,
            current_pipeline,
            recorded: None,
            count: 0,
            current: Buffer::create(device, 80, BufferUsage::vertex(), "CurrentCameraFrustum"),
        }
    }
    pub fn load(&mut self, device: &Device, instances: &[FrustumInstance]) {
        self.recorded = Some(Buffer::from_data(
            device,
            instances,
            BufferUsage::vertex(),
            "RecordedCameraFrustums",
        ));
        self.count = instances.len() as u32;
    }
    pub fn update_current(&self, device: &Device, instance: FrustumInstance) {
        self.current.write(device, &[instance], 0);
    }
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(recorded) = &self.recorded {
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, recorded.gpu().slice(..));
            pass.draw(0..24, 0..self.count);
            pass.set_pipeline(&self.current_pipeline);
            pass.set_vertex_buffer(0, self.current.gpu().slice(..));
            pass.draw(0..72, 0..1);
        }
    }
}
