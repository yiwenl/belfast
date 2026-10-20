use belfast::{BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device};
use glam::{Mat4, Vec3};
use serde::Deserialize;

use crate::{
    model::{ModelRenderer, PointLight},
    model_data::ModelData,
    shadow::ShadowMap,
};

pub const PARTICLE_COUNT: u32 = 10_000;
pub const PARTICLE_COUNT_OPTIONS: [u32; 3] = [10_000, 100_000, 500_000];
pub const SPAWN_JITTER: f32 = 0.03;
pub const SIZE_MULTIPLIER_RANGE: [f32; 2] = [0.45, 1.65];
pub const SPEED_MULTIPLIER_RANGE: [f32; 2] = [0.6, 1.6];
pub const WIND_ACCELERATION: [f32; 3] = [0.12, 0.22, -0.04];
/// Velocity retained per 1/60 second, applied after integrating position.
pub const FRICTION: f32 = 0.985;
pub const HOLD_FRACTION: f32 = 0.20;
pub const SCALE_IN_FRACTION: f32 = 0.10;
pub const SCALE_OUT_FRACTION: f32 = 0.20;
pub const CURL_SCALE: f32 = 1.5;
pub const CURL_STRENGTH: f32 = 0.08;
pub const CURL_SPEED: f32 = 0.18;
pub const LIFETIME_RANGE: [f32; 2] = [6., 12.];
/// World-space allowance for curl wandering, calibrated for this scene's noise.
/// Recheck the GPU flight test when changing noise, lifetime or speed settings.
pub const SHADOW_CURL_MARGIN: f32 = 1.5;
const STATE_SHADER: &str = include_str!("petal-state.wgsl");
const NOISE_SHADER: &str = include_str!("petal-noise.wgsl");
const UPDATE_SHADER: &str = include_str!("petal-update.wgsl");
const DRAW_SHADER: &str = include_str!("petal-draw.wgsl");
const TRANSFORM_SHADER: &str = include_str!("petal-transform.wgsl");
const SHADOW_SHADER: &str = include_str!("petal-shadow.wgsl");

#[cfg(test)]
#[path = "petals_tests.rs"]
mod tests;

/// Five vec4s, shared verbatim with petal-state.wgsl (80-byte storage stride).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position_life: [f32; 4],
    pub velocity_angle: [f32; 4],
    pub origin_lifetime: [f32; 4],
    pub axis_speed: [f32; 4],
    pub appearance: [f32; 4], // base scale, initial angle, lifetime scale, movement speed
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmitterFile {
    schema_version: u32,
    coordinate_system: Coordinates,
    counts: Counts,
    emitters: Vec<Emitter>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Coordinates {
    space: String,
    handedness: String,
    up_axis: String,
    front_axis: String,
    units: String,
}
#[derive(Deserialize)]
struct Counts {
    total: usize,
}
#[derive(Deserialize)]
struct Emitter {
    position: [f32; 3],
}

pub fn parse_emitters(bytes: &[u8]) -> Result<Vec<Vec3>, String> {
    let file: EmitterFile =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid magnolia emitter JSON: {e}"))?;
    let c = file.coordinate_system;
    if file.schema_version != 1
        || c.space != "tree-local"
        || c.handedness != "right"
        || c.up_axis != "+Y"
        || c.front_axis != "+Z"
        || c.units != "meters"
    {
        return Err("Magnolia emitters must use schema 1 and right-handed tree-local +Y-up / +Z-forward meters".into());
    }
    if file.emitters.is_empty() || file.emitters.len() != file.counts.total {
        return Err("Magnolia emitter count is empty or does not match counts.total".into());
    }
    file.emitters
        .into_iter()
        .map(|emitter| {
            let position = Vec3::from(emitter.position);
            if position.is_finite() {
                Ok(position)
            } else {
                Err("Emitter positions must be finite".into())
            }
        })
        .collect()
}

/// A stable seed makes initial layout repeatable when comparing rendering modes.
pub fn initial_particles(
    emitters: &[Vec3],
    transform: Mat4,
    count: u32,
) -> Result<Vec<Particle>, String> {
    let scale = transform.x_axis.truncate().length();
    if emitters.is_empty()
        || emitters.iter().any(|p| !p.is_finite())
        || count == 0
        || count > 1_000_000
        || !transform.is_finite()
        || scale <= 0.
    {
        return Err(
            "Petals need finite emitters, a positive transform scale and 1–1,000,000 particles"
                .into(),
        );
    }
    let mut seed = 0x6d61_676eu32;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 8) as f32 / 16_777_216.
    };
    let mut particles = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let jitter = Vec3::new(random() - 0.5, random() - 0.5, random() - 0.5)
            * (SPAWN_JITTER * 2. / 3f32.sqrt());
        let origin = transform.transform_point3(emitters[i % emitters.len()]) + jitter;
        let lifetime = LIFETIME_RANGE[0] + random() * (LIFETIME_RANGE[1] - LIFETIME_RANGE[0]);
        // Start across the whole cycle so most petals can move on the first frame.
        let age = random();
        let remaining = lifetime * (1. - age);
        // Uniform random direction on the sphere (no zero-vector normalization).
        let z = random() * 2. - 1.;
        let azimuth = random() * std::f32::consts::TAU;
        let r = (1. - z * z).max(0.).sqrt();
        let axis = Vec3::new(r * azimuth.cos(), r * azimuth.sin(), z);
        let speed = (0.6 + random() * 1.4) * if random() < 0.5 { -1. } else { 1. };
        let angle = random() * std::f32::consts::TAU;
        particles.push(Particle {
            position_life: [origin.x, origin.y, origin.z, remaining],
            velocity_angle: [0., 0., 0., angle],
            origin_lifetime: [origin.x, origin.y, origin.z, lifetime],
            axis_speed: [axis.x, axis.y, axis.z, speed],
            appearance: [
                scale
                    * (SIZE_MULTIPLIER_RANGE[0]
                        + random() * (SIZE_MULTIPLIER_RANGE[1] - SIZE_MULTIPLIER_RANGE[0])),
                angle,
                lifetime_scale(age),
                SPEED_MULTIPLIER_RANGE[0]
                    + random() * (SPEED_MULTIPLIER_RANGE[1] - SPEED_MULTIPLIER_RANGE[0]),
            ],
        });
    }
    Ok(particles)
}

// Match the compute shader's envelope before the first (possibly zero-dt) draw.
fn lifetime_scale(age: f32) -> f32 {
    let smooth = |t: f32| {
        let t = t.clamp(0., 1.);
        t * t * (3. - 2. * t)
    };
    smooth(age / SCALE_IN_FRACTION)
        * (1. - smooth((age - (1. - SCALE_OUT_FRACTION)) / SCALE_OUT_FRACTION))
}

/// Stable shadow framing: wind travel plus a practical curl displacement margin.
/// This is a quality envelope, not the theoretical maximum of sustained curl force.
pub fn motion_bounds(emitters: &[Vec3], transform: Mat4, model: &ModelData) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for emitter in emitters {
        let p = transform.transform_point3(*emitter);
        min = min.min(p);
        max = max.max(p);
    }
    let radius = model
        .primitives
        .iter()
        .flat_map(|p| &p.vertices)
        .map(|v| Vec3::from(v.position).length())
        .fold(0f32, f32::max)
        * transform.x_axis.truncate().length()
        * SIZE_MULTIPLIER_RANGE[1];
    let wind = Vec3::from(WIND_ACCELERATION);
    let active_time = LIFETIME_RANGE[1] * (1. - HOLD_FRACTION);
    let damping = -60. * FRICTION.ln();
    // Post-friction speed <= force/damping; allow one pre-friction Euler step.
    let travel = (1. / damping + 1. / 120.) * active_time * SPEED_MULTIPLIER_RANGE[1];
    // Keep the whole tree, all emitter locations, full-size rotating petals and
    // a curl wandering margin. Do not multiply peak curl by an entire lifetime:
    // that made a >20 m box for a tree under 3 m tall, wasting most shadow texels.
    let padding = Vec3::splat(radius + SPAWN_JITTER + SHADOW_CURL_MARGIN);
    (
        min + wind.min(Vec3::ZERO) * travel - padding,
        max + wind.max(Vec3::ZERO) * travel + padding,
    )
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SimParams {
    wind_friction: [f32; 4],
    noise: [f32; 4],
    lifecycle: [f32; 4],
    delta_seconds: f32,
    simulation_time: f32,
    count: u32,
    padding: u32,
}

pub struct PetalSystem {
    model: ModelRenderer,
    compute: Compute,
    params: Buffer,
    render_params: Buffer,
    draw_layout: wgpu::BindGroupLayout,
    buffers: ParticleBuffers,
    current: usize,
    count: u32,
    simulation_time: f32,
    pending_dt: f32,
}

struct ParticleBuffers {
    compute_bindings: [BindGroup; 2],
    draw_bindings: [wgpu::BindGroup; 2],
    // Bind groups retain the storage buffers on GPU; keep handles for ownership clarity.
    states: [Buffer; 2],
}

impl ParticleBuffers {
    fn new(
        device: &Device,
        particles: &[Particle],
        params: &Buffer,
        render_params: &Buffer,
        compute_layout: &wgpu::BindGroupLayout,
        draw_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let size = std::mem::size_of_val(particles) as u64;
        if particles.is_empty()
            || size > device.gpu().limits().max_storage_buffer_binding_size
            || size > device.gpu().limits().max_buffer_size
            || particles.len().div_ceil(64)
                > device.gpu().limits().max_compute_workgroups_per_dimension as usize
        {
            return Err("Petal state is empty or exceeds GPU buffer limits".into());
        }
        let states = ["PetalsA", "PetalsB"].map(|label| {
            Buffer::from_data(
                device,
                particles,
                BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
                label,
            )
        });
        let compute_bindings = [0, 1].map(|read| {
            BindGroup::create(
                device,
                compute_layout,
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: states[read].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: states[1 - read].gpu().as_entire_binding(),
                    },
                ],
                "PetalComputeBinding",
            )
        });
        let draw_bindings = [0, 1].map(|index| {
            device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("PetalDrawBinding"),
                layout: draw_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: states[index].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: render_params.gpu().as_entire_binding(),
                    },
                ],
            })
        });
        Ok(Self {
            compute_bindings,
            draw_bindings,
            states,
        })
    }
}

impl PetalSystem {
    pub fn new(device: &Device, data: &ModelData, particles: &[Particle]) -> Result<Self, String> {
        Self::build(device, data, particles, None)
    }

    pub fn new_shadowed(
        device: &Device,
        data: &ModelData,
        particles: &[Particle],
        shadow: &ShadowMap,
    ) -> Result<Self, String> {
        Self::build(device, data, particles, Some(shadow))
    }

    fn build(
        device: &Device,
        data: &ModelData,
        particles: &[Particle],
        shadow: Option<&ShadowMap>,
    ) -> Result<Self, String> {
        let params = Buffer::create(
            device,
            std::mem::size_of::<SimParams>() as u64,
            BufferUsage::uniform(),
            "PetalSimParams",
        );
        let compute = Compute::new(
            device,
            &format!("{STATE_SHADER}\n{NOISE_SHADER}\n{UPDATE_SHADER}"),
            ComputeOptions::new("PetalUpdate"),
        );
        // Global size is independent of simulation and survives particle resets.
        let render_params = Buffer::from_data(
            device,
            &[0.5f32, 0., 0., 0.],
            BufferUsage::uniform(),
            "PetalRenderParams",
        );
        let draw_layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PetalInstancesLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let buffers = ParticleBuffers::new(
            device,
            particles,
            &params,
            &render_params,
            &compute.get_bind_group_layout(0),
            &draw_layout,
        )?;
        let model = ModelRenderer::new_instanced(
            device,
            data,
            &draw_layout,
            &format!("{STATE_SHADER}\n{TRANSFORM_SHADER}\n{DRAW_SHADER}"),
            &format!(
                "{}\n{STATE_SHADER}\n{TRANSFORM_SHADER}\n{SHADOW_SHADER}",
                crate::shadow::DEPTH_SHADER
            ),
            shadow,
        )?;
        Ok(Self {
            model,
            compute,
            params,
            render_params,
            draw_layout,
            buffers,
            current: 0,
            count: particles.len() as u32,
            simulation_time: 0.,
            pending_dt: 0.,
        })
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn set_scale(&self, device: &Device, scale: f32) -> Result<f32, String> {
        if !scale.is_finite() || !(0.2..=1.0).contains(&scale) {
            return Err("Petal scale must be between 0.2 and 1.0".into());
        }
        self.render_params.write(device, &[scale, 0., 0., 0.], 0);
        Ok(scale)
    }

    /// Keep mesh, textures and pipelines; replace only the particle state and bindings.
    pub fn reset_particles(
        &mut self,
        device: &Device,
        particles: &[Particle],
    ) -> Result<(), String> {
        let buffers = ParticleBuffers::new(
            device,
            particles,
            &self.params,
            &self.render_params,
            &self.compute.get_bind_group_layout(0),
            &self.draw_layout,
        )?;
        self.buffers = buffers;
        self.current = 0;
        self.count = particles.len() as u32;
        self.pending_dt = 0.;
        self.write_sim_params(device);
        Ok(())
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, view_projection: &[f32; 16]) {
        self.model.update_camera(device, view_projection);
        self.pending_dt = if delta_seconds.is_finite() {
            delta_seconds.clamp(0., 0.1)
        } else {
            0.
        };
        self.write_sim_params(device);
    }

    fn write_sim_params(&self, device: &Device) {
        self.params.write(
            device,
            &[SimParams {
                wind_friction: [
                    WIND_ACCELERATION[0],
                    WIND_ACCELERATION[1],
                    WIND_ACCELERATION[2],
                    FRICTION,
                ],
                noise: [CURL_SCALE, CURL_STRENGTH, CURL_SPEED, 0.],
                lifecycle: [HOLD_FRACTION, SCALE_IN_FRACTION, SCALE_OUT_FRACTION, 0.],
                delta_seconds: self.pending_dt,
                simulation_time: self.simulation_time,
                count: self.count,
                padding: 0,
            }],
            0,
        );
    }

    pub fn update_light(&self, device: &Device, light: PointLight) {
        self.model.update_light(device, light);
    }

    /// Encode before drawing, then point the renderer at the just-written buffer.
    pub fn simulate(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.compute.run(
            encoder,
            Some(&self.buffers.compute_bindings[self.current]),
            [self.count.div_ceil(64), 1, 1],
            Some("PetalSimulation"),
        );
        self.current = 1 - self.current;
        self.simulation_time += self.pending_dt;
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        debug_assert!(self.buffers.states[self.current].size() > 0);
        self.model.draw_instances(
            pass,
            Some(&self.buffers.draw_bindings[self.current]),
            self.count,
        );
    }

    pub fn draw_shadow<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.model.draw_shadow_instances(
            pass,
            Some(&self.buffers.draw_bindings[self.current]),
            self.count,
        );
    }
}
