use crate::vortex_debug::VortexDebug;
use belfast::{BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device};
use glam::Vec3;

pub const STATE_SHADER: &str = include_str!("petal-state.wgsl");
const COMMON_SHADER: &str = include_str!("vortex-common.wgsl");
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
    origin_radius: [f32; 4],
    step: [f32; 4],
    filament: [f32; 4], // Core radius, upward drift speed, XZ drift speed, reserved.
    counts: [u32; 4],
}

pub struct VortexFlow {
    pub states: [Buffer; 2],
    pub current: usize,
    filaments: [Buffer; 2],
    debug: VortexDebug,
    filament_step: Compute,
    particle_step: Compute,
    filament_bindings: [BindGroup; 2],
    particle_bindings: [BindGroup; 2],
    params: Buffer,
    origin_radius: [f32; 4],
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
        let filaments = std::array::from_fn(|_| {
            Buffer::create(
                device,
                u64::from(RINGS * VERTICES_PER_RING) * 16,
                BufferUsage::storage(),
                "VortexFilaments",
            )
        });
        let params = Buffer::create(
            device,
            std::mem::size_of::<FlowParams>() as u64,
            BufferUsage::uniform(),
            "VortexParams",
        );
        let filament_step = Compute::new(
            device,
            &format!("{COMMON_SHADER}\n{}", include_str!("vortex-filaments.wgsl")),
            ComputeOptions::new("VortexFilamentUpdate"),
        );
        let particle_step = Compute::new(
            device,
            &format!(
                "{STATE_SHADER}\n{COMMON_SHADER}\n{}",
                include_str!("petal-update.wgsl")
            ),
            ComputeOptions::new("VortexPetalUpdate"),
        );
        let filament_bindings = std::array::from_fn(|i| {
            BindGroup::create(
                device,
                &filament_step.get_bind_group_layout(0),
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: filaments[i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: filaments[1 - i].gpu().as_entire_binding(),
                    },
                ],
                "VortexFilamentBinding",
            )
        });
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
                        resource: filaments[1 - i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: states[i].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: states[1 - i].gpu().as_entire_binding(),
                    },
                ],
                "VortexPetalBinding",
            )
        });
        let debug = VortexDebug::new(device, camera, &filaments, VERTICES_PER_RING)?;
        Ok(Self {
            states,
            current: 0,
            filaments,
            debug,
            filament_step,
            particle_step,
            filament_bindings,
            particle_bindings,
            params,
            origin_radius: [0.0; 4],
            time: 0.0,
            pending_dt: 0.0,
            count: 0,
        })
    }

    pub fn reset(&mut self, device: &Device, bounds: (Vec3, Vec3)) {
        let center = (bounds.0 + bounds.1) * 0.5;
        let origin = Vec3::new(center.x, bounds.0.y - 0.12, center.z);
        self.origin_radius = origin.extend(MotionSettings::default().ring_radius).to_array();
        self.seed_filaments(device);
        self.current = 0;
        self.time = 0.0;
        self.pending_dt = 0.0;
    }

    fn seed_filaments(&self, device: &Device) {
        let [x, y, z, radius] = self.origin_radius;
        let origin = Vec3::new(x, y, z);
        let vertices: Vec<[f32; 4]> = (0..RINGS * VERTICES_PER_RING)
            .map(|index| {
                let ring = index / VERTICES_PER_RING;
                let angle = (index % VERTICES_PER_RING) as f32 * std::f32::consts::TAU
                    / VERTICES_PER_RING as f32;
                let age = ring as f32;
                let offset = Vec3::new((ring as f32 * 2.4).sin() * radius * 0.5, age * 0.12, 0.0);
                let position =
                    origin + offset + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                position.extend(age).to_array()
            })
            .collect();
        for buffer in &self.filaments {
            buffer.write(device, &vertices, 0);
        }
    }

    pub fn prepare(
        &mut self,
        device: &Device,
        dt: f32,
        count: u32,
        settings: MotionSettings,
        shadow_view_projection: [f32; 16],
    ) {
        if self.origin_radius[3] != settings.ring_radius {
            self.origin_radius[3] = settings.ring_radius;
            // Resize rings immediately without rewinding the current petal buffer.
            self.seed_filaments(device);
        }
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
                origin_radius: self.origin_radius,
                step: [
                    self.pending_dt,
                    self.time,
                    settings.petal_updraft,
                    settings.strength,
                ],
                filament: [
                    settings.core_radius,
                    settings.filament_updraft,
                    settings.filament_drift,
                    0.0,
                ],
                counts: [count, RINGS, VERTICES_PER_RING, 0],
            }],
            0,
        );
    }

    pub fn simulate(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.pending_dt <= 0.0 {
            return;
        }
        self.filament_step.run(
            encoder,
            Some(&self.filament_bindings[self.current]),
            [(RINGS * VERTICES_PER_RING).div_ceil(64), 1, 1],
            Some("VortexFilaments"),
        );
        self.particle_step.run(
            encoder,
            Some(&self.particle_bindings[self.current]),
            [self.count.div_ceil(64), 1, 1],
            Some("PetalAdvection"),
        );
        self.current = 1 - self.current;
        self.time += self.pending_dt;
        self.pending_dt = 0.0;
    }

    pub fn draw_debug<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.debug.draw(pass, self.current, RINGS);
    }
}
