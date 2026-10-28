use belfast::{
    fluid::FluidSimulation, BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device,
};

pub const COUNT: u32 = 256 * 256;
pub const STATE: &str = include_str!("particle-state.wgsl");
pub fn shader_source() -> String {
    format!("{STATE}\n{}", include_str!("particle-update.wgsl"))
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 4], // xyz, life
    pub velocity: [f32; 4],
    pub origin: [f32; 4], // xyz, life decay
    pub extra: [f32; 4],  // size, stretch, speed, brightness
}

pub fn initial_particles(count: u32) -> Vec<Particle> {
    let mut seed = 0x62b10c5u32;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 8) as f32 / 16777216.0
    };
    (0..count)
        .map(|_| {
            let origin = [
                random() * 8.0 - 4.0,
                (random() * 2.0 - 1.0) * 2.25,
                (random() * 2.0 - 1.0) * 0.1,
                0.24 + random() * 0.48,
            ];
            let size = if random() < 0.00025 {
                8.0 + random() * 2.0
            } else {
                1.0 + random() * 2.0
            };
            Particle {
                position: [origin[0], origin[1], origin[2], random()],
                velocity: [0.0; 4],
                origin,
                extra: [
                    size,
                    2.0 + random() * 2.0,
                    1.0 + random(),
                    0.9 + random() * 0.1,
                ],
            }
        })
        .collect()
}

pub struct Particles {
    pub buffers: [Buffer; 2],
    pub current: usize,
    compute: Compute,
    groups: [BindGroup; 2],
    params: Buffer,
}
impl Particles {
    pub fn new(device: &Device, fluid: &FluidSimulation) -> Self {
        let initial = initial_particles(COUNT);
        let buffers = std::array::from_fn(|_| {
            Buffer::from_data(device, &initial, BufferUsage::storage(), "BlocksParticles")
        });
        let params = Buffer::create(device, 32, BufferUsage::uniform(), "BlocksSimulationParams");
        let compute = Compute::new(
            device,
            &shader_source(),
            ComputeOptions::new("BlocksCompute"),
        );
        let groups = std::array::from_fn(|i| {
            BindGroup::create(
                device,
                &compute.get_bind_group_layout(0),
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers[i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffers[1 - i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: fluid.field().gpu().as_entire_binding(),
                    },
                ],
                "BlocksComputeBinding",
            )
        });
        Self {
            buffers,
            current: 0,
            compute,
            groups,
            params,
        }
    }
    pub fn step(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        time: f32,
        response: f32,
        depth: f32,
        fluid_size: u32,
    ) {
        let data = [
            1.0 / 60.0,
            time,
            COUNT as f32,
            fluid_size as f32,
            response,
            depth,
            0.0,
            0.0,
        ];
        let upload = Buffer::from_data(
            device,
            &data,
            wgpu::BufferUsages::COPY_SRC,
            "BlocksStepUpload",
        );
        encoder.copy_buffer_to_buffer(upload.gpu(), 0, self.params.gpu(), 0, 32);
        self.compute.run(
            encoder,
            Some(&self.groups[self.current]),
            [COUNT.div_ceil(64), 1, 1],
            Some("BlocksSimulation"),
        );
        self.current = 1 - self.current;
    }
    pub fn reset(&mut self, device: &Device) {
        let initial = initial_particles(COUNT);
        for buffer in &self.buffers {
            buffer.write(device, &initial, 0);
        }
        self.current = 0;
    }
}
