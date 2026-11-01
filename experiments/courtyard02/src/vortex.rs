use crate::vortex_debug::VortexDebug;
use belfast::{
    filament::{FilamentSettings, FilamentSimulation, WGSL_FILAMENT},
    BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device,
};
use glam::Vec3;

pub const STATE_SHADER: &str = include_str!("petal-state.wgsl");
const RINGS: u32 = 6;
const VERTICES_PER_RING: u32 = 12;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 4],
    pub rotation: [f32; 4],
    pub velocity: [f32; 4],
    pub origin: [f32; 4],
    pub spin: [f32; 4],
}

#[derive(Clone, Copy)]
pub struct MotionSettings {
    pub enabled: bool,
    pub petal_updraft: f32,
    pub filament_updraft: f32,
    pub filament_drift: f32,
    pub ring_radius: f32,
    pub core_radius: f32,
    pub strength: f32,
}

impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            petal_updraft: 0.05,
            filament_updraft: 0.18,
            filament_drift: 0.06,
            ring_radius: 0.21,
            core_radius: 0.036,
            strength: 1.61,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FlowParams {
    shadow_view_projection: [f32; 16],
    step: [f32; 4],
    counts: [u32; 4],
}

pub struct VortexFlow {
    pub states: [Buffer; 2],
    pub current: usize,
    filaments: FilamentSimulation,
    debug: VortexDebug,
    particle_step: Compute,
    particle_bindings: [BindGroup; 2],
    params: Buffer,
    origin: [f32; 3],
    pending_reset: bool,
    time: f32,
    pending_dt: f32,
    count: u32,
}

impl VortexFlow {
    pub fn new(device: &Device, capacity: u32, camera: &Buffer) -> Result<Self, String> {
        let bytes = u64::from(capacity) * std::mem::size_of::<Particle>() as u64;
        let limits = device.gpu().limits();
        if bytes > limits.max_buffer_size
            || bytes > u64::from(limits.max_storage_buffer_binding_size)
        {
            return Err("Petal simulation exceeds GPU storage limits".into());
        }
        let states = std::array::from_fn(|_| {
            Buffer::create(device, bytes, BufferUsage::storage(), "PetalState")
        });
        let filaments = FilamentSimulation::new(
            device,
            RINGS,
            VERTICES_PER_RING,
            FilamentSettings::default(),
        )?;
        let params = Buffer::create(
            device,
            std::mem::size_of::<FlowParams>() as u64,
            BufferUsage::uniform(),
            "VortexParams",
        );
        let particle_step = Compute::new(
            device,
            &format!(
                "{STATE_SHADER}\n{WGSL_FILAMENT}\n{}",
                include_str!("petal-update.wgsl")
            ),
            ComputeOptions::new("VortexPetalUpdate"),
        );
        let particle_bindings = std::array::from_fn(|i| {
            BindGroup::create(
                device,
                &particle_step.get_bind_group_layout(0),
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.gpu().as_entire_binding(),
                    },
                    // Petals follow the filament positions just written by the first pass.
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: filaments.field().gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: states[i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: states[1 - i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: filaments.params().gpu().as_entire_binding(),
                    },
                ],
                "VortexPetalBinding",
            )
        });
        let debug = VortexDebug::new(device, camera, &filaments)?;
        Ok(Self {
            states,
            current: 0,
            filaments,
            debug,
            particle_step,
            particle_bindings,
            params,
            origin: [0.0; 3],
            pending_reset: false,
            time: 0.0,
            pending_dt: 0.0,
            count: 0,
        })
    }

    pub fn reset(&mut self, bounds: (Vec3, Vec3)) {
        let center = (bounds.0 + bounds.1) * 0.5;
        self.origin = [center.x, bounds.0.y - 0.12, center.z];
        self.pending_reset = true;
        self.current = 0;
        self.time = 0.0;
        self.pending_dt = 0.0;
    }

    pub fn prepare(
        &mut self,
        device: &Device,
        dt: f32,
        count: u32,
        settings: MotionSettings,
        shadow_view_projection: [f32; 16],
    ) -> Result<(), String> {
        self.filaments.set_settings(FilamentSettings {
            origin: self.origin,
            ring_radius: settings.ring_radius,
            core_radius: settings.core_radius,
            updraft: settings.filament_updraft,
            xz_drift: settings.filament_drift,
            strength: settings.strength,
            ..Default::default()
        })?;
        // Avoid a large jump after a suspended tab; rendering and shadows use the same step.
        self.pending_dt = if settings.enabled && dt.is_finite() {
            dt.clamp(0.0, 1.0 / 30.0)
        } else {
            0.0
        };
        self.count = count;
        self.params.write(
            device,
            &[FlowParams {
                shadow_view_projection,
                step: [self.pending_dt, self.time, settings.petal_updraft, 0.0],
                counts: [count, 0, 0, 0],
            }],
            0,
        );
        Ok(())
    }

    pub fn simulate(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), String> {
        if self.pending_reset {
            self.filaments.reset(encoder);
            self.pending_reset = false;
        }
        // Flush radius/settings changes even while particle animation is paused.
        self.filaments.step(device, encoder, self.pending_dt)?;
        if self.pending_dt <= 0.0 {
            return Ok(());
        }
        self.particle_step.run(
            encoder,
            Some(&self.particle_bindings[self.current]),
            [self.count.div_ceil(64), 1, 1],
            Some("PetalAdvection"),
        );
        self.current = 1 - self.current;
        self.time += self.pending_dt;
        self.pending_dt = 0.0;
        Ok(())
    }

    pub fn draw_debug<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.debug.draw(pass, self.filaments.rings());
    }
}
