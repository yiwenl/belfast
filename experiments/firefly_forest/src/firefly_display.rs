use super::trails::FRAMES;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderMode {
    #[default]
    Particles,
    Trails,
    Drag,
}

pub(super) struct Display {
    mode: RenderMode,
    dragging: bool,
    mix: f32,
    age: f32,
    release_mix: f32,
    release_age: f32,
    release_elapsed: f32,
    boost_mix: f32,
    boost_release_rate: f32,
    flash_recovery: f32,
}

const FADE_OUT: f32 = 0.25;
const RETRACT: f32 = 0.65;

impl Default for Display {
    fn default() -> Self {
        Self {
            mode: RenderMode::default(),
            dragging: false,
            mix: 0.,
            age: 0.,
            release_mix: 0.,
            release_age: 0.,
            release_elapsed: 0.,
            boost_mix: 0.,
            boost_release_rate: 0.,
            flash_recovery: RETRACT,
        }
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3. - 2. * t)
}

impl Display {
    pub(super) fn set_mode(&mut self, mode: RenderMode) {
        if self.mode != mode {
            self.mode = mode;
            self.reset();
        }
    }

    pub(super) fn set_dragging(&mut self, dragging: bool) {
        if self.dragging == dragging {
            return;
        }
        self.dragging = dragging;
        if !dragging {
            self.release_mix = self.mix;
            self.release_age = self.age;
            self.release_elapsed = 0.;
            self.boost_release_rate = self.boost_mix / self.flash_recovery;
        }
    }

    pub(super) fn set_flash_recovery(&mut self, seconds: f32) -> Result<(), String> {
        if !seconds.is_finite() {
            return Err("Firefly flash recovery must be finite".into());
        }
        let seconds = seconds.clamp(0.1, 5.);
        if seconds != self.flash_recovery {
            self.flash_recovery = seconds;
            // Retarget from the current brightness without relighting or jumping.
            if !self.dragging {
                self.boost_release_rate = self.boost_mix / seconds;
            }
        }
        Ok(())
    }

    pub(super) fn reset(&mut self) {
        self.mix = if self.mode == RenderMode::Trails {
            1.
        } else {
            0.
        };
        self.age = self.mix * (FRAMES - 1) as f32;
        self.release_mix = 0.;
        self.release_age = 0.;
        self.release_elapsed = RETRACT;
        self.boost_mix = self.mix;
        self.boost_release_rate = 0.;
    }

    pub(super) fn update(&mut self, dt: f32) {
        if self.mode != RenderMode::Drag {
            return;
        }
        if self.dragging {
            self.mix = (self.mix + dt / FADE_OUT).min(1.);
            self.boost_mix = (self.boost_mix + dt / FADE_OUT).min(1.);
            self.age = (self.age + 1.).min((FRAMES - 1) as f32);
        } else {
            self.release_elapsed = (self.release_elapsed + dt).min(RETRACT);
            let t = self.release_elapsed / RETRACT;
            self.mix = self.release_mix * (1. - t);
            self.boost_mix = (self.boost_mix - dt * self.boost_release_rate).max(0.);
            // Keep recording the moving head; trim the tail along that live curve.
            self.age = self.release_age * (1. - smooth(t));
        }
    }

    pub(super) fn particle_opacity(&self) -> f32 {
        1. - smooth(self.mix)
    }

    pub(super) fn flash_boost(&self) -> f32 {
        smooth(self.boost_mix)
    }

    pub(super) fn movement_rate(&self, configured: f32) -> f32 {
        if self.mode == RenderMode::Drag {
            configured + (1. - configured) * smooth(self.mix)
        } else {
            configured
        }
    }

    pub(super) fn max_age(&self) -> f32 {
        self.age
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_recovery_is_independent_of_trails_motion_and_frame_rate() {
        let recover = |fps: u32| {
            let mut display = Display::default();
            display.set_mode(RenderMode::Drag);
            display.set_flash_recovery(2.).unwrap();
            display.set_dragging(true);
            for _ in 0..fps {
                display.update(1. / fps as f32);
            }
            display.set_dragging(false);
            for _ in 0..fps {
                display.update(1. / fps as f32);
            }
            assert_eq!(display.max_age(), 0.);
            assert_eq!(display.particle_opacity(), 1.);
            assert_eq!(display.movement_rate(0.27), 0.27);
            let boost = display.flash_boost();
            assert!((boost - 0.5).abs() < 1e-5);
            for _ in 0..fps + 1 {
                display.update(1. / fps as f32);
            }
            assert_eq!(display.flash_boost(), 0.);
            boost
        };
        assert!((recover(30) - recover(120)).abs() < 1e-5);
    }

    #[test]
    fn changing_recovery_and_regrabbing_preserve_brightness() {
        let mut display = Display::default();
        display.set_mode(RenderMode::Drag);
        display.set_dragging(true);
        display.update(0.1);
        display.set_dragging(false);
        display.update(0.1);
        let boost = display.flash_boost();
        display.set_flash_recovery(3.).unwrap();
        assert_eq!(display.flash_boost(), boost);
        display.update(0.1);
        assert!(display.flash_boost() < boost && display.flash_boost() > 0.);
        let boost = display.flash_boost();
        display.set_dragging(true);
        assert_eq!(display.flash_boost(), boost);
        display.update(0.05);
        assert!(display.flash_boost() > boost);
        assert!(display.set_flash_recovery(f32::NAN).is_err());
        assert!(display.set_flash_recovery(f32::INFINITY).is_err());
        display.set_flash_recovery(-1.).unwrap();
        display.set_dragging(false);
        display.update(0.1);
        assert_eq!(display.flash_boost(), 0.);
        display.set_mode(RenderMode::Trails);
        assert_eq!(display.flash_boost(), 1.);
        display.set_mode(RenderMode::Particles);
        assert_eq!(display.flash_boost(), 0.);
    }

    #[test]
    fn drag_grows_from_head_and_release_retracts_while_particles_fade_back() {
        let mut display = Display::default();
        display.set_mode(RenderMode::Drag);
        for _ in 0..300 {
            display.update(1. / 60.);
        }
        assert_eq!(display.max_age(), 0.);
        assert_eq!(display.particle_opacity(), 1.);
        display.set_dragging(true);
        display.update(1. / 60.);
        assert_eq!(display.max_age(), 1.);
        assert!((0.9..1.).contains(&display.particle_opacity()));
        for _ in 1..180 {
            display.update(1. / 60.);
        }
        assert_eq!(display.max_age(), 180.);
        assert_eq!(display.particle_opacity(), 0.);
        display.set_dragging(false);
        let mut age = display.max_age();
        for _ in 0..40 {
            display.update(1. / 60.);
            assert!(display.max_age() <= age);
            age = display.max_age();
        }
        assert_eq!(age, 0.);
        assert_eq!(display.particle_opacity(), 1.);
    }

    #[test]
    fn regrabbing_and_reapplying_mode_do_not_pop_or_restore_old_history() {
        let mut display = Display::default();
        display.set_mode(RenderMode::Drag);
        display.set_dragging(true);
        for _ in 0..60 {
            display.update(1. / 60.);
        }
        display.set_dragging(false);
        display.update(0.1);
        let age = display.max_age();
        let opacity = display.particle_opacity();
        display.set_dragging(false);
        display.set_mode(RenderMode::Drag);
        display.set_dragging(true);
        assert_eq!(display.max_age(), age);
        assert_eq!(display.particle_opacity(), opacity);
        display.update(1. / 60.);
        assert!((display.max_age() - age - 1.).abs() < 1e-5);
        assert!(display.particle_opacity() < opacity);
        display.reset();
        assert_eq!(display.max_age(), 0.);
        assert_eq!(display.particle_opacity(), 1.);
    }

    #[test]
    fn fade_duration_is_independent_of_render_rate_and_manual_modes_still_work() {
        let release = |fps: u32| {
            let mut display = Display::default();
            display.set_mode(RenderMode::Drag);
            display.set_dragging(true);
            for _ in 0..300 {
                display.update(1. / fps as f32);
            }
            assert_eq!(display.max_age(), 255.);
            display.set_dragging(false);
            for _ in 0..fps / 2 {
                display.update(1. / fps as f32);
            }
            (display.max_age(), display.particle_opacity())
        };
        let a = release(30);
        let b = release(120);
        assert!((a.0 - b.0).abs() < 0.001);
        assert!((a.1 - b.1).abs() < 0.0001);
        let mut display = Display::default();
        display.set_mode(RenderMode::Trails);
        display.update(0.1);
        assert_eq!(display.max_age(), 255.);
        assert_eq!(display.particle_opacity(), 0.);
        display.set_mode(RenderMode::Particles);
        display.set_dragging(true);
        display.update(0.1);
        assert_eq!(display.max_age(), 0.);
        assert_eq!(display.particle_opacity(), 1.);
    }
}
