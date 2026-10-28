use crate::{
    motion::Motion,
    mouse::MouseFlow,
    particles::Particles,
    renderer::{RenderOptions, Renderer},
};
use belfast::{
    fluid::{FluidSettings, FluidSimulation, FluidSplat, MAX_SPLATS},
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
    pub mouse_strength: f32,
    pub mouse_radius: f32,
    pub hand_strength: f32,
    pub hand_radius: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            flow_strength: 14.0,
            flow_radius: 1.0,
            dissipation: 0.985,
            pressure_iterations: 20,
            response: 1.0,
            depth_motion: 2.0,
            block_size: 1.0,
            shadow_strength: 0.75,
            mouse_strength: 50.0,
            mouse_radius: 0.08,
            hand_strength: 70.0,
            hand_radius: 0.2,
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
            (self.mouse_strength, 0.0, 200.0),
            (self.mouse_radius, 0.01, 0.3),
            (self.hand_strength, 0.0, 200.0),
            (self.hand_radius, 0.01, 0.3),
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
    mouse: MouseFlow,
    hands: [MouseFlow; 2],
    motion: Motion,
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
            show_debug_maps: false,
            mouse: MouseFlow::default(),
            hands: std::array::from_fn(|_| MouseFlow::default()),
            motion: Motion::default(),
            clock: FixedClock::default(),
            time: 0.0,
        })
    }
    pub fn set_settings(&mut self, settings: Settings) -> Result<(), String> {
        settings.validate()?;
        self.fluid.set_settings(FluidSettings {
            velocity_dissipation: settings.dissipation,
            density_dissipation: settings.dissipation,
            pressure_dissipation: settings.dissipation,
            pressure_iterations: settings.pressure_iterations,
        })?;
        self.settings = settings;
        Ok(())
    }
    pub fn set_particle_count(&mut self, count: u32) -> Result<(), String> {
        self.particles.set_count(count)
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
        self.mouse.clear();
        self.clear_hands();
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
        self.mouse.clear();
        self.clear_hands();
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        self.fluid.reset(&mut encoder);
        device.queue().submit([encoder.finish()]);
        self.particles.reset(device);
        self.clock = FixedClock::default();
        self.time = 0.0;
        self.motion.restart();
    }
    pub fn set_motion(&mut self, kind: &str, values: &[f32]) -> Result<bool, String> {
        self.motion.set(kind, values)
    }
    pub fn set_motion_enabled(&mut self, enabled: bool) {
        self.motion.set_enabled(enabled);
    }
    pub fn motion_gain(&self) -> f32 {
        self.motion.gain()
    }
    pub fn is_color_transitioning(&self) -> bool {
        self.renderer.is_color_transitioning()
    }
    pub fn set_color_map(
        &mut self,
        device: &Device,
        texture: belfast::Texture,
    ) -> Result<(), String> {
        self.renderer.set_color_map(device, texture)
    }
    pub fn pointer_move(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if !self.paused {
            self.mouse.move_to(&self.camera, x, y, width, height);
        }
    }
    pub fn clear_pointer(&mut self) {
        self.mouse.clear();
    }
    pub fn hand_move(&mut self, hand: usize, x: f32, y: f32, width: f32, height: f32) {
        if !self.paused {
            if let Some(flow) = self.hands.get_mut(hand) {
                flow.move_to(&self.camera, x, y, width, height);
            }
        }
    }
    pub fn clear_hand(&mut self, hand: usize) {
        if let Some(flow) = self.hands.get_mut(hand) {
            flow.clear();
        }
    }
    fn clear_hands(&mut self) {
        for hand in &mut self.hands {
            hand.clear();
        }
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.mouse.clear();
            self.clear_hands();
        }
        self.paused = paused;
    }
    pub fn frame(
        &mut self,
        device: &Device,
        view: &wgpu::TextureView,
        dt: f32,
    ) -> Result<(), String> {
        // Palette transitions continue when particle motion is paused.
        self.renderer.advance_color_transition(dt);
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        for _ in 0..self.clock.advance(dt, self.paused) {
            self.time += 1.0 / 60.0;
            let gain = self.motion.step(1.0 / 60.0);
            let mut splats = emitters(self.time, &self.settings, gain).to_vec();
            // Share the remaining splats fairly between the mouse and both palms.
            let moving_inputs = usize::from(self.mouse.has_motion())
                + self.hands.iter().filter(|hand| hand.has_motion()).count();
            let per_input = (MAX_SPLATS - splats.len()) / moving_inputs.max(1);
            self.mouse.append_splats(
                self.settings.mouse_strength,
                self.settings.mouse_radius,
                per_input,
                &mut splats,
            );
            for hand in &mut self.hands {
                hand.append_splats(
                    self.settings.hand_strength,
                    self.settings.hand_radius,
                    per_input,
                    &mut splats,
                );
            }
            self.fluid.step(device, &mut encoder, 1.0 / 60.0, &splats)?;
            self.particles.step(
                device,
                &mut encoder,
                self.time,
                self.settings.response * gain,
                self.settings.depth_motion * gain,
                self.fluid.size(),
            );
        }
        self.renderer.render(
            device,
            &mut encoder,
            view,
            &self.particles,
            &self.camera,
            RenderOptions {
                size: self.settings.block_size,
                shadow_strength: self.settings.shadow_strength,
                show_debug_maps: self.show_debug_maps,
            },
        );
        device.queue().submit([encoder.finish()]);
        Ok(())
    }
}
fn emitters(time: f32, settings: &Settings, gain: f32) -> [FluidSplat; 8] {
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
        let strength = settings.flow_strength / 60.0 * gain;
        FluidSplat {
            position,
            velocity: [
                angle.cos() * direction * strength,
                angle.sin() * direction * strength,
            ],
            radius: (0.1 + 0.04 * (time * 1.3 + phase).sin()) * settings.flow_radius,
            density: 0.05 * gain,
        }
    })
}
