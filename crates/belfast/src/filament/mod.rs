//! Reusable Y-up vortex rings with GPU self-advection, drift and staggered respawning.
//!
//! Like `fluid::FluidSimulation`, this owns only the simulation. `field()` is a stable
//! storage buffer of ring-major `vec4<f32>` vertices: world XYZ and age in seconds.
//! Encode `step` before consumers in the same command stream; there is no GPU readback.
//! See the module README for bindings and the WGSL velocity sampler.
use crate::{BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device};
use glam::Vec3;

/// Append to a consumer shader declaring `filament_params: FilamentParams` as a
/// uniform and `filament_vertices: array<vec4<f32>>` as read-only storage, at any
/// binding locations. Call `filament_velocity(world_position)` for world units/second.
pub const WGSL_FILAMENT: &str = include_str!("sampling.wgsl");

#[derive(Clone, Copy, Debug)]
pub struct FilamentSettings {
    pub origin: [f32; 3],
    pub ring_radius: f32,
    pub core_radius: f32,
    /// Independent +Y translation, in world units/second.
    pub updraft: f32,
    /// Coherent horizontal drift per ring, in world units/second.
    pub xz_drift: f32,
    pub strength: f32,
    pub circulation: f32,
    /// Ring lifetime in seconds; initial ages are evenly staggered across it.
    pub lifetime: f32,
}

impl Default for FilamentSettings {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            ring_radius: 0.21,
            core_radius: 0.036,
            updraft: 0.18,
            xz_drift: 0.06,
            strength: 1.61,
            circulation: 0.003,
            lifetime: 6.0,
        }
    }
}

impl FilamentSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.origin.iter().all(|v| v.is_finite())
            || ![self.ring_radius, self.core_radius, self.lifetime]
                .iter()
                .all(|v| v.is_finite() && *v > 0.0 && (v * v).is_finite() && v * v > 0.0)
            || ![self.updraft, self.xz_drift, self.strength, self.circulation]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0)
            || !(self.strength * self.circulation).is_finite()
        {
            return Err("Filaments need a finite origin, positive radii/lifetime, and finite nonnegative drift/strength/circulation".into());
        }
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FilamentParams {
    origin_radius: [f32; 4],
    step: [f32; 4],   // dt, time, strength, lifetime
    motion: [f32; 4], // core radius, upward drift, XZ drift, circulation
    counts: [u32; 4], // rings, vertices per ring, padding
}

pub struct FilamentSimulation {
    device: Device,
    ring_capacity: u32,
    rings: u32,
    vertices_per_ring: u32,
    settings: FilamentSettings,
    field: Buffer,
    scratch: Buffer,
    params: Buffer,
    advect: Compute,
    binding: BindGroup,
    time: f32,
    reseed: bool,
}

impl FilamentSimulation {
    pub fn new(
        device: &Device,
        rings: u32,
        vertices_per_ring: u32,
        settings: FilamentSettings,
    ) -> Result<Self, String> {
        settings.validate()?;
        let count = rings
            .checked_mul(vertices_per_ring)
            .ok_or("Filament vertex count overflows buffer addressing")?;
        let bytes = u64::from(count) * 16;
        let limits = device.gpu().limits();
        if rings == 0
            || vertices_per_ring < 3
            || bytes > limits.max_storage_buffer_binding_size
            || bytes > limits.max_buffer_size
            || count.div_ceil(64) > limits.max_compute_workgroups_per_dimension
        {
            return Err("Filaments need at least one ring with three vertices and must fit GPU storage/dispatch limits".into());
        }
        let storage = BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC;
        let field = Buffer::from_data(
            device,
            &seed_vertices(rings, vertices_per_ring, settings),
            storage,
            "FilamentField",
        );
        let scratch = Buffer::create(device, bytes, storage, "FilamentScratch");
        let params = Buffer::from_data(
            device,
            &[settings.encode(rings, vertices_per_ring, 0.0, 0.0)],
            BufferUsage::uniform(),
            "FilamentParams",
        );
        let advect = Compute::new(
            device,
            &format!("{WGSL_FILAMENT}\n{}", include_str!("solver.wgsl")),
            ComputeOptions::new("FilamentAdvection"),
        );
        let binding = BindGroup::create(
            device,
            &advect.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: field.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scratch.gpu().as_entire_binding(),
                },
            ],
            "FilamentBinding",
        );
        Ok(Self {
            device: device.clone(),
            ring_capacity: rings,
            rings,
            vertices_per_ring,
            settings,
            field,
            scratch,
            params,
            advect,
            binding,
            time: 0.0,
            reseed: false,
        })
    }

    pub fn rings(&self) -> u32 {
        self.rings
    }

    /// Reseed this many active rings on the next step, keeping consumer bindings stable.
    pub fn set_ring_count(&mut self, rings: u32) -> Result<(), String> {
        if !(1..=self.ring_capacity).contains(&rings) {
            return Err(format!(
                "Ring count must be between 1 and {}",
                self.ring_capacity
            ));
        }
        self.reseed |= self.rings != rings;
        self.rings = rings;
        Ok(())
    }

    pub fn vertices_per_ring(&self) -> u32 {
        self.vertices_per_ring
    }
    pub fn field(&self) -> &Buffer {
        &self.field
    }
    /// Stable 64-byte uniform consumed alongside `field()` by `WGSL_FILAMENT`.
    pub fn params(&self) -> &Buffer {
        &self.params
    }

    /// Applies on the next `step`, including dt=0. Origin/radius/lifetime changes
    /// reseed the rings without rewinding the drift clock or any consumer particles.
    pub fn set_settings(&mut self, settings: FilamentSettings) -> Result<(), String> {
        settings.validate()?;
        self.reseed |= settings.origin != self.settings.origin
            || settings.ring_radius != self.settings.ring_radius
            || settings.lifetime != self.settings.lifetime;
        self.settings = settings;
        Ok(())
    }

    /// Encode one bounded step, then consume `field()` before the next step if
    /// intermediate results are needed. Multiple steps may share one encoder.
    pub fn step(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        dt: f32,
    ) -> Result<(), String> {
        if !self.device.is_same(device) {
            return Err("Filament device mismatch".into());
        }
        if !dt.is_finite() || !(0.0..=1.0 / 30.0).contains(&dt) {
            return Err("Filament step needs finite dt in 0..1/30 seconds".into());
        }
        if self.reseed {
            self.seed(encoder);
        }
        self.upload_params(encoder, dt);
        if dt == 0.0 {
            return Ok(());
        }
        self.advect.run(
            encoder,
            Some(&self.binding),
            [(self.rings * self.vertices_per_ring).div_ceil(64), 1, 1],
            Some("FilamentAdvection"),
        );
        // Keep consumer bindings stable while all vertices read the same previous state.
        encoder.copy_buffer_to_buffer(
            self.scratch.gpu(),
            0,
            self.field.gpu(),
            0,
            u64::from(self.rings * self.vertices_per_ring) * 16,
        );
        self.time += dt;
        Ok(())
    }

    /// Restore staggered rings using the current settings and rewind the clock.
    /// Submit this encoder before later work that consumes the simulation.
    pub fn reset(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.time = 0.0;
        self.seed(encoder);
        self.upload_params(encoder, 0.0);
    }

    fn seed(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let upload = Buffer::from_data(
            &self.device,
            &seed_vertices(self.rings, self.vertices_per_ring, self.settings),
            wgpu::BufferUsages::COPY_SRC,
            "FilamentSeedUpload",
        );
        encoder.copy_buffer_to_buffer(upload.gpu(), 0, self.field.gpu(), 0, upload.size());
        self.reseed = false;
    }

    fn upload_params(&self, encoder: &mut wgpu::CommandEncoder, dt: f32) {
        // Encoder-ordered uploads prevent later steps overwriting earlier step parameters.
        let upload = Buffer::from_data(
            &self.device,
            &[self
                .settings
                .encode(self.rings, self.vertices_per_ring, dt, self.time)],
            wgpu::BufferUsages::COPY_SRC,
            "FilamentStepUpload",
        );
        encoder.copy_buffer_to_buffer(upload.gpu(), 0, self.params.gpu(), 0, self.params.size());
    }
}

impl FilamentSettings {
    fn encode(self, rings: u32, vertices: u32, dt: f32, time: f32) -> FilamentParams {
        FilamentParams {
            origin_radius: [
                self.origin[0],
                self.origin[1],
                self.origin[2],
                self.ring_radius,
            ],
            step: [dt, time, self.strength, self.lifetime],
            motion: [
                self.core_radius,
                self.updraft,
                self.xz_drift,
                self.circulation,
            ],
            counts: [rings, vertices, 0, 0],
        }
    }
}

fn seed_vertices(rings: u32, vertices_per_ring: u32, settings: FilamentSettings) -> Vec<[f32; 4]> {
    let origin = Vec3::from_array(settings.origin);
    let radius = settings.ring_radius;
    (0..rings * vertices_per_ring)
        .map(|index| {
            let ring = index / vertices_per_ring;
            let angle = (index % vertices_per_ring) as f32 * std::f32::consts::TAU
                / vertices_per_ring as f32;
            let age = ring as f32 * (settings.lifetime / rings as f32);
            let offset = Vec3::new((ring as f32 * 2.4).sin() * radius * 0.5, age * 0.12, 0.0);
            (origin + offset + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius))
                .extend(age)
                .to_array()
        })
        .collect()
}
