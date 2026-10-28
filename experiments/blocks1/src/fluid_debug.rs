use belfast::{
    fluid::FluidSimulation, BindGroup, Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh,
};

pub struct FluidDebug {
    draw: Draw,
    mesh: Mesh,
    binding: BindGroup,
    viewport: Buffer,
    grid_size: u32,
}

impl FluidDebug {
    pub fn new(device: &Device, fluid: &FluidSimulation) -> Result<Self, String> {
        let mesh = Mesh::new(6).map_err(|e| e.to_string())?;
        let draw = Draw::new(
            device,
            include_str!("fluid-debug.wgsl"),
            &mesh,
            DrawOptions::new("FluidDebug", device.format()),
        );
        let viewport = Buffer::create(device, 16, BufferUsage::uniform(), "FluidDebugViewport");
        let binding = BindGroup::create(
            device,
            &draw.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fluid.field().gpu().as_entire_binding(),
                },
            ],
            "FluidDebugBinding",
        );
        Ok(Self {
            draw,
            mesh,
            binding,
            viewport,
            grid_size: fluid.size(),
        })
    }

    pub fn resize(&self, device: &Device, width: u32, height: u32, pixel_ratio: f32) {
        self.viewport.write(
            device,
            &[
                width as f32,
                height as f32,
                pixel_ratio,
                self.grid_size as f32,
            ],
            0,
        );
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("FluidDebugOverlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        self.binding.bind(&mut pass, 0);
        self.draw.draw(&mut pass, &self.mesh, 2);
    }
}
