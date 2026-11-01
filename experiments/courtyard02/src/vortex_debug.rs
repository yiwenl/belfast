use belfast::{filament::FilamentSimulation, BindGroup, Buffer, Device, Draw, DrawOptions, Mesh};

pub struct VortexDebug {
    mesh: Mesh,
    draw: Draw,
    binding: BindGroup,
}

impl VortexDebug {
    pub fn new(
        device: &Device,
        camera: &Buffer,
        filaments: &FilamentSimulation,
    ) -> Result<Self, String> {
        let vertices_per_ring = filaments.vertices_per_ring();
        let mesh = Mesh::new(vertices_per_ring * 2).map_err(|error| error.to_string())?;
        let source = format!(
            "const VERTICES_PER_RING: u32 = {vertices_per_ring}u;\n{}",
            include_str!("vortex-debug.wgsl")
        );
        let mut options = DrawOptions::new("VortexDebug", device.format());
        options.primitive.topology = wgpu::PrimitiveTopology::LineList;
        let draw = Draw::new(device, &source, &mesh, options);
        let binding = BindGroup::create(
            device,
            &draw.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: filaments.field().gpu().as_entire_binding(),
                },
            ],
            "VortexDebugBinding",
        );
        Ok(Self {
            mesh,
            draw,
            binding,
        })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, rings: u32) {
        self.binding.bind(pass, 0);
        self.draw.draw(pass, &self.mesh, rings);
    }
}
