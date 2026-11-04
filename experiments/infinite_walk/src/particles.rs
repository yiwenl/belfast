use belfast::{
    filament::{FilamentSettings, FilamentSimulation, WGSL_FILAMENT},
    BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device, Draw, DrawOptions, Mesh,
};

const MAX_COUNT: u32 = 500_000;
pub const STRIDE: u64 = 48;
pub const MAX_RINGS: u32 = 24;
const VERTICES_PER_RING: u32 = 12;
const CAMERA: &str = include_str!("camera.wgsl");
const STATE: &str = include_str!("particle-state.wgsl");

#[derive(Clone, Copy)]
pub struct ParticleSettings {
    pub count: u32,
    pub visible: bool,
    pub animate: bool,
    pub size: f32,
    pub lifetime: f32,
    pub life_variation: f32,
    pub outside_decay: f32,
    pub lift: f32,
    pub drag: f32,
    pub ground_y: f32,
}

impl Default for ParticleSettings {
    fn default() -> Self {
        Self {
            count: 119_565,
            visible: true,
            animate: true,
            size: 0.014,
            lifetime: 5.0,
            life_variation: 0.4,
            outside_decay: 2.0,
            lift: 0.18,
            drag: 1.6,
            ground_y: -0.11,
        }
    }
}

impl ParticleSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.count > MAX_COUNT {
            return Err(format!("Particle count must be between 0 and {MAX_COUNT}"));
        }
        for (value, min, max) in [
            (self.size, 0.001, 0.05),
            (self.lifetime, 0.5, 20.0),
            (self.life_variation, 0.0, 0.9),
            (self.outside_decay, 1.0, 10.0),
            (self.lift, 0.0, 1.0),
            (self.drag, 0.1, 8.0),
            (self.ground_y, -2.0, 3.0),
        ] {
            if !(min..=max).contains(&value) {
                return Err("Particle setting outside its supported range".into());
            }
        }
        Ok(())
    }
}

pub fn filament_defaults() -> FilamentSettings {
    FilamentSettings {
        ring_radius: 0.69,
        core_radius: 0.08,
        updraft: 0.54,
        xz_drift: 0.08,
        strength: 2.34,
        circulation: 0.031,
        ..Default::default()
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Motion {
    step: [f32; 4],
    life: [f32; 4],
    counts: [u32; 4],
}

pub struct Particles {
    states: [Buffer; 2],
    active_count: u32,
    current: usize,
    filaments: FilamentSimulation,
    spawns: Buffer,
    source_count: u32,
    select_spawns: Compute,
    spawn_binding: BindGroup,
    advect: Compute,
    bindings: [BindGroup; 2],
    motion: Buffer,
    debug_mesh: Mesh,
    debug_draw: Draw,
    debug_binding: BindGroup,
}

impl Particles {
    pub fn new(
        device: &Device,
        sources: &Buffer,
        source_count: u32,
        camera: &Buffer,
    ) -> Result<Self, String> {
        let limits = device.gpu().limits();
        // Each forest point can emit from both its original and its offset copy.
        let spawn_bytes = 16 + u64::from(source_count) * 2 * 16;
        if [sources.size(), spawn_bytes, u64::from(MAX_COUNT) * STRIDE]
            .into_iter()
            .any(|bytes| {
                bytes > limits.max_buffer_size || bytes > limits.max_storage_buffer_binding_size
            })
            || source_count.max(MAX_COUNT).div_ceil(64)
                > limits.max_compute_workgroups_per_dimension
        {
            return Err("Particle emitters exceed GPU storage or dispatch limits".into());
        }
        let states = std::array::from_fn(|_| {
            Buffer::create(
                device,
                u64::from(MAX_COUNT) * STRIDE,
                BufferUsage::vertex_storage(),
                "WalkParticleState",
            )
        });
        let spawns = Buffer::create(
            device,
            spawn_bytes,
            BufferUsage::storage(),
            "WalkParticleSpawns",
        );
        let filaments =
            FilamentSimulation::new(device, MAX_RINGS, VERTICES_PER_RING, filament_defaults())?;
        let motion = Buffer::create(
            device,
            std::mem::size_of::<Motion>() as u64,
            BufferUsage::uniform(),
            "WalkParticleMotion",
        );
        let select_spawns = Compute::new(
            device,
            &format!("{CAMERA}\n{STATE}\n{}", include_str!("particle-spawn.wgsl")),
            ComputeOptions::new("WalkSelectSpawns"),
        );
        let spawn_binding = BindGroup::create(
            device,
            &select_spawns.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sources.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: spawns.gpu().as_entire_binding(),
                },
            ],
            "WalkSpawnBinding",
        );
        let advect = Compute::new(
            device,
            &format!(
                "{CAMERA}\n{STATE}\n{WGSL_FILAMENT}\n{}",
                include_str!("particle-update.wgsl")
            ),
            ComputeOptions::new("WalkParticleAdvection"),
        );
        let bindings = std::array::from_fn(|i| {
            BindGroup::create(
                device,
                &advect.get_bind_group_layout(0),
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera.gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: filaments.field().gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: filaments.params().gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: states[i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: states[1 - i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: spawns.gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: motion.gpu().as_entire_binding(),
                    },
                ],
                "WalkParticleBinding",
            )
        });
        let debug_mesh = Mesh::new(VERTICES_PER_RING * 2).map_err(|error| error.to_string())?;
        let mut options = DrawOptions::new("WalkFilamentDebug", device.format());
        options.primitive.topology = wgpu::PrimitiveTopology::LineList;
        options.depth_stencil = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let debug_draw = Draw::new(
            device,
            &format!(
                "const VERTICES_PER_RING: u32 = {VERTICES_PER_RING}u;\n{CAMERA}\n{STATE}\n{}",
                include_str!("filament-debug.wgsl")
            ),
            &debug_mesh,
            options,
        );
        let debug_binding = BindGroup::create(
            device,
            &debug_draw.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: filaments.field().gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: motion.gpu().as_entire_binding(),
                },
            ],
            "WalkFilamentDebugBinding",
        );
        Ok(Self {
            states,
            active_count: 0,
            current: 0,
            filaments,
            spawns,
            source_count,
            select_spawns,
            spawn_binding,
            advect,
            bindings,
            motion,
            debug_mesh,
            debug_draw,
            debug_binding,
        })
    }

    pub fn simulate(
        &mut self,
        device: &Device,
        dt: f32,
        settings: ParticleSettings,
        filament: FilamentSettings,
        ring_count: u32,
    ) -> Result<(), String> {
        let dt = if settings.animate {
            dt.clamp(0.0, 1.0 / 30.0)
        } else {
            0.0
        };
        self.filaments.set_settings(filament)?;
        self.filaments.set_ring_count(ring_count)?;
        self.motion.write(
            device,
            &[Motion {
                step: [dt, settings.size, settings.lift, settings.drag],
                life: [
                    settings.lifetime,
                    settings.life_variation,
                    settings.ground_y,
                    settings.outside_decay,
                ],
                counts: [settings.count, self.active_count, 0, 0],
            }],
            0,
        );
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("WalkParticleSimulation"),
            });
        self.filaments.step(device, &mut encoder, dt)?;
        self.active_count = settings.count;
        if self.active_count == 0 {
            device.queue().submit([encoder.finish()]);
            return Ok(());
        }
        encoder.clear_buffer(self.spawns.gpu(), 0, Some(16));
        self.select_spawns.run(
            &mut encoder,
            Some(&self.spawn_binding),
            [self.source_count.div_ceil(64), 1, 1],
            Some("WalkSelectSpawns"),
        );
        self.advect.run(
            &mut encoder,
            Some(&self.bindings[self.current]),
            [self.active_count.div_ceil(64), 1, 1],
            Some("WalkParticleAdvection"),
        );
        self.current = 1 - self.current;
        device.queue().submit([encoder.finish()]);
        Ok(())
    }

    pub fn buffer(&self) -> &Buffer {
        &self.states[self.current]
    }

    pub fn count(&self) -> u32 {
        self.active_count
    }

    pub fn draw_debug<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.debug_binding.bind(pass, 0);
        self.debug_draw
            .draw(pass, &self.debug_mesh, self.filaments.rings());
    }
}
