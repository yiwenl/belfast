//! Reusable square 2D stable-fluid solver. No rendering, emitters or input handlers.
//!
//! `field()` is a stable storage binding of row-major `vec4<f32>` cells:
//! `(velocity_u, velocity_v, 0, density)`. UV and velocity use an upward Y axis;
//! velocity is UV/second. Encode `step` before consumers in the same command stream.
//! Sampling mirrors at the grid edges without flipping velocity or imposing walls.
use crate::{BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device};

pub const MAX_SPLATS: usize = 32;
const SHADER: &str = include_str!("solver.wgsl");

#[derive(Clone, Copy, Debug)]
pub struct FluidSettings {
    /// Retention per 1/60 second, in [0, 1].
    pub velocity_dissipation: f32,
    pub density_dissipation: f32,
    /// Retention of the previous pressure solution per 1/60 second, in [0, 1].
    pub pressure_dissipation: f32,
    pub pressure_iterations: u32,
}
impl Default for FluidSettings {
    fn default() -> Self {
        Self {
            velocity_dissipation: 0.985,
            density_dissipation: 0.985,
            pressure_dissipation: 0.985,
            pressure_iterations: 20,
        }
    }
}
impl FluidSettings {
    pub fn validate(&self) -> Result<(), String> {
        if ![
            self.velocity_dissipation,
            self.density_dissipation,
            self.pressure_dissipation,
        ]
        .iter()
        .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
            || !(1..=80).contains(&self.pressure_iterations)
        {
            return Err("Fluid retention must be 0..1 and pressure iterations 1..80".into());
        }
        Ok(())
    }
}

/// An impulse, not a rate. Multiply continuous emitter strengths by dt at the call site.
#[derive(Clone, Copy, Debug)]
pub struct FluidSplat {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub radius: f32,
    pub density: f32,
}
impl FluidSplat {
    fn validate(&self) -> bool {
        self.position
            .iter()
            .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
            && self
                .velocity
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 100.0)
            && self.radius.is_finite()
            && self.radius > 0.0
            && self.radius <= 1.0
            && self.density.is_finite()
            && (0.0..=10.0).contains(&self.density)
    }
}

pub struct FluidSimulation {
    device: Device,
    size: u32,
    settings: FluidSettings,
    fields: [Buffer; 2],
    pressure: [Buffer; 2],
    pressure_read: usize,
    divergence: Buffer,
    params: Buffer,
    advect: Compute,
    divergence_pass: Compute,
    warm_pressure: Compute,
    jacobi: Compute,
    project: Compute,
    advect_group: BindGroup,
    divergence_group: BindGroup,
    warm_pressure_groups: [BindGroup; 2],
    jacobi_groups: [BindGroup; 2],
    project_groups: [BindGroup; 2],
}
impl FluidSimulation {
    pub fn new(device: &Device, size: u32, settings: FluidSettings) -> Result<Self, String> {
        settings.validate()?;
        let limits = device.gpu().limits();
        let bytes = u64::from(size)
            .checked_mul(u64::from(size))
            .and_then(|n| n.checked_mul(16))
            .ok_or("Fluid grid size overflows buffer addressing")?;
        if size < 4
            || bytes > limits.max_storage_buffer_binding_size
            || bytes > limits.max_buffer_size
            || size.div_ceil(8) > limits.max_compute_workgroups_per_dimension
        {
            return Err(
                "Fluid grid must be at least 4 and fit the device storage/dispatch limits".into(),
            );
        }
        let storage = BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC;
        let fields = std::array::from_fn(|_| Buffer::create(device, bytes, storage, "FluidField"));
        let pressure =
            std::array::from_fn(|_| Buffer::create(device, bytes / 4, storage, "FluidPressure"));
        let divergence = Buffer::create(device, bytes / 4, storage, "FluidDivergence");
        let params = Buffer::create(
            device,
            (32 + MAX_SPLATS * 32) as u64,
            BufferUsage::uniform(),
            "FluidParams",
        );
        let pipeline = |entry_point| {
            Compute::new(
                device,
                SHADER,
                ComputeOptions {
                    label: entry_point,
                    layout: None,
                    entry_point,
                },
            )
        };
        let advect = pipeline("advect");
        let divergence_pass = pipeline("divergence_main");
        let warm_pressure = pipeline("warm_pressure");
        let jacobi = pipeline("jacobi");
        let project = pipeline("project");
        let group = |pipeline: &Compute, buffers: &[(u32, &Buffer)]| {
            let entries: Vec<_> = buffers
                .iter()
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding: *binding,
                    resource: buffer.gpu().as_entire_binding(),
                })
                .collect();
            BindGroup::create(
                device,
                &pipeline.get_bind_group_layout(0),
                &entries,
                "FluidBinding",
            )
        };
        let advect_group = group(&advect, &[(0, &params), (1, &fields[0]), (2, &fields[1])]);
        let divergence_group = group(
            &divergence_pass,
            &[(0, &params), (1, &fields[1]), (5, &divergence)],
        );
        let jacobi_groups = std::array::from_fn(|i| {
            group(
                &jacobi,
                &[
                    (0, &params),
                    (3, &pressure[i]),
                    (4, &pressure[1 - i]),
                    (5, &divergence),
                ],
            )
        });
        let warm_pressure_groups = std::array::from_fn(|i| {
            group(
                &warm_pressure,
                &[(0, &params), (3, &pressure[i]), (4, &pressure[1 - i])],
            )
        });
        let project_groups = std::array::from_fn(|i| {
            group(
                &project,
                &[
                    (0, &params),
                    (1, &fields[1]),
                    (2, &fields[0]),
                    (3, &pressure[i]),
                ],
            )
        });
        Ok(Self {
            device: device.clone(),
            size,
            settings,
            fields,
            pressure,
            pressure_read: 0,
            divergence,
            params,
            advect,
            divergence_pass,
            warm_pressure,
            jacobi,
            project,
            advect_group,
            divergence_group,
            warm_pressure_groups,
            jacobi_groups,
            project_groups,
        })
    }
    pub fn size(&self) -> u32 {
        self.size
    }
    pub fn field(&self) -> &Buffer {
        &self.fields[0]
    }
    pub fn set_settings(&mut self, settings: FluidSettings) -> Result<(), String> {
        settings.validate()?;
        self.settings = settings;
        Ok(())
    }
    /// Encode one bounded step. Multiple steps can safely share an encoder.
    pub fn step(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        dt: f32,
        splats: &[FluidSplat],
    ) -> Result<(), String> {
        if !self.device.is_same(device) {
            return Err("Fluid device mismatch".into());
        }
        if !dt.is_finite()
            || !(0.0..=1.0 / 30.0).contains(&dt)
            || splats.len() > MAX_SPLATS
            || !splats.iter().all(FluidSplat::validate)
        {
            return Err("Fluid step needs dt 0..1/30 and at most 32 finite, bounded splats".into());
        }
        if dt == 0.0 {
            return Ok(());
        }
        let mut data = vec![0.0f32; 8 + MAX_SPLATS * 8];
        data[..8].copy_from_slice(&[
            self.size as f32,
            dt,
            self.settings.velocity_dissipation.powf(dt * 60.0),
            self.settings.density_dissipation.powf(dt * 60.0),
            splats.len() as f32,
            self.settings.pressure_dissipation.powf(dt * 60.0),
            0.0,
            0.0,
        ]);
        for (i, splat) in splats.iter().enumerate() {
            data[8 + i * 8..16 + i * 8].copy_from_slice(&[
                splat.position[0],
                splat.position[1],
                splat.velocity[0],
                splat.velocity[1],
                splat.radius,
                splat.density,
                0.0,
                0.0,
            ]);
        }
        // Encoder-ordered uploads prevent later steps overwriting earlier dispatch uniforms.
        let upload = Buffer::from_data(
            device,
            &data,
            wgpu::BufferUsages::COPY_SRC,
            "FluidStepUpload",
        );
        encoder.copy_buffer_to_buffer(upload.gpu(), 0, self.params.gpu(), 0, self.params.size());
        let groups = [self.size.div_ceil(8), self.size.div_ceil(8), 1];
        self.advect.run(
            encoder,
            Some(&self.advect_group),
            groups,
            Some("FluidAdvection"),
        );
        self.divergence_pass.run(
            encoder,
            Some(&self.divergence_group),
            groups,
            Some("FluidDivergence"),
        );
        self.warm_pressure.run(
            encoder,
            Some(&self.warm_pressure_groups[self.pressure_read]),
            groups,
            Some("FluidPressureRetention"),
        );
        self.pressure_read = 1 - self.pressure_read;
        for _ in 0..self.settings.pressure_iterations {
            self.jacobi.run(
                encoder,
                Some(&self.jacobi_groups[self.pressure_read]),
                groups,
                Some("FluidPressure"),
            );
            self.pressure_read = 1 - self.pressure_read;
        }
        self.project.run(
            encoder,
            Some(&self.project_groups[self.pressure_read]),
            groups,
            Some("FluidProjection"),
        );
        Ok(())
    }
    pub fn reset(&self, encoder: &mut wgpu::CommandEncoder) {
        for buffer in self
            .fields
            .iter()
            .chain(self.pressure.iter())
            .chain(std::iter::once(&self.divergence))
        {
            encoder.clear_buffer(buffer.gpu(), 0, None);
        }
    }
}
