use crate::{particles::Particles, renderer::Renderer};
use belfast::{
    fluid::{FluidSettings, FluidSimulation, FluidSplat},
    Device, PerspectiveCamera,
};

#[derive(Clone, Copy)]
pub struct Settings {
    pub flow_strength: f32,
    pub flow_radius: f32,
    pub dissipation: f32,
    pub pressure_iterations: u32,
    pub response: f32,
    pub depth_motion: f32,
    pub block_size: f32,
    pub shadow_strength: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            flow_strength: 14.0,
            flow_radius: 1.0,
            dissipation: 0.985,
            pressure_iterations: 20,
            response: 1.0,
            depth_motion: 1.0,
            block_size: 1.0,
            shadow_strength: 0.75,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        for (v, min, max) in [
            (self.flow_strength, 0.0, 40.0),
            (self.flow_radius, 0.25, 2.0),
            (self.dissipation, 0.9, 1.0),
            (self.response, 0.0, 3.0),
            (self.depth_motion, 0.0, 3.0),
            (self.block_size, 0.2, 3.0),
            (self.shadow_strength, 0.0, 1.0),
        ] {
            if !v.is_finite() || !(min..=max).contains(&v) {
                return Err("Blocks setting is non-finite or outside its supported range".into());
            }
        }
        if !(1..=80).contains(&self.pressure_iterations) {
            return Err("Pressure iterations must be 1..80".into());
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct FixedClock {
    accumulated: f32,
}
impl FixedClock {
    pub fn advance(&mut self, dt: f32, paused: bool) -> u32 {
        if paused || !dt.is_finite() || dt < 0.0 {
            self.accumulated = 0.0;
            return 0;
        }
        self.accumulated = (self.accumulated + dt).min(4.0 / 60.0);
        let steps = (self.accumulated * 60.0 + 0.00001).floor() as u32;
        self.accumulated = (self.accumulated - steps as f32 / 60.0).max(0.0);
        steps
    }
}
pub struct Scene {
    pub camera: PerspectiveCamera,
    fluid: FluidSimulation,
    particles: Particles,
    renderer: Renderer,
    settings: Settings,
    pub paused: bool,
    pub show_debug_maps: bool,
    clock: FixedClock,
    time: f32,
}
impl Scene {
    pub fn new(device: &Device) -> Result<Self, String> {
        let fluid = FluidSimulation::new(device, 128, FluidSettings::default())?;
        let particles = Particles::new(device, &fluid);
        let renderer = Renderer::new(device, &particles, &fluid)?;
        let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 50.0);
        camera.look_at([0.0, 0.0, 12.0], [0.0, 0.0, 0.0]);
        Ok(Self {
            camera,
            fluid,
            particles,
            renderer,
            settings: Settings::default(),
            paused: false,
            show_debug_maps: true,
            clock: FixedClock::default(),
            time: 0.0,
        })
    }
    pub fn set_settings(&mut self, settings: Settings) -> Result<(), String> {
        settings.validate()?;
        self.fluid.set_settings(FluidSettings {
            velocity_dissipation: settings.dissipation,
            density_dissipation: settings.dissipation,
            pressure_iterations: settings.pressure_iterations,
        })?;
        self.settings = settings;
        Ok(())
    }
    pub fn resize(
        &mut self,
        device: &Device,
        width: u32,
        height: u32,
        pixel_ratio: f32,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err("Scene dimensions must be positive".into());
        }
        let radius = if width >= height {
            12.0
        } else {
            12.0 * height as f32 / width as f32
        };
        self.camera = PerspectiveCamera::new(
            std::f32::consts::FRAC_PI_4,
            width as f32 / height as f32,
            0.1,
            radius + 30.0,
        );
        self.camera.look_at([0.0, 0.0, radius], [0.0, 0.0, 0.0]);
        self.renderer
            .resize(device, width, height, pixel_ratio)
            .map_err(|e| e.to_string())
    }
    pub fn reset(&mut self, device: &Device) {
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        self.fluid.reset(&mut encoder);
        device.queue().submit([encoder.finish()]);
        self.particles.reset(device);
        self.clock = FixedClock::default();
        self.time = 0.0;
    }
    pub fn frame(
        &mut self,
        device: &Device,
        view: &wgpu::TextureView,
        dt: f32,
    ) -> Result<(), String> {
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        for _ in 0..self.clock.advance(dt, self.paused) {
            self.time += 1.0 / 60.0;
            let splats = emitters(self.time, &self.settings);
            self.fluid.step(device, &mut encoder, 1.0 / 60.0, &splats)?;
            self.particles.step(
                device,
                &mut encoder,
                self.time,
                self.settings.response,
                self.settings.depth_motion,
                self.fluid.size(),
            );
        }
        self.renderer.render(
            device,
            &mut encoder,
            view,
            &self.particles,
            &self.camera,
            self.settings.block_size,
            self.settings.shadow_strength,
            self.show_debug_maps,
        );
        device.queue().submit([encoder.finish()]);
        Ok(())
    }
}
fn emitters(time: f32, settings: &Settings) -> [FluidSplat; 8] {
    std::array::from_fn(|i| {
        let j = (i % 4) as f32;
        let t = time * 0.5;
        let phase = i as f32 * 2.399;
        let angle = (time * 1.7 + phase).sin() * 0.5 + (time * 0.73 + phase * 1.3).sin() * 0.15;
        let direction = if i < 4 { 1.0 } else { -1.0 };
        let position = if i < 4 {
            [(j + 0.5) / 4.0, (j * 2.0 + t * 0.8).sin() * 0.25 + 0.5]
        } else {
            [j / 4.0, (j * 1.5 + t * 1.2).cos() * 0.25 + 0.5]
        };
        let strength = settings.flow_strength / 60.0;
        FluidSplat {
            position,
            velocity: [
                angle.cos() * direction * strength,
                angle.sin() * direction * strength,
            ],
            radius: (0.1 + 0.04 * (time * 1.3 + phase).sin()) * settings.flow_radius,
            density: 0.05,
        }
    })
}
