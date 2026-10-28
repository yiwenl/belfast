struct Profile {
    base: f32,
    peak: f32,
    interval: f32,
    duration: f32,
}

pub struct Motion {
    profile: Option<Profile>,
    enabled: bool,
    elapsed: f32,
    gain: f32,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            profile: None,
            enabled: true,
            elapsed: 0.0,
            gain: 1.0,
        }
    }
}

impl Motion {
    pub fn set(&mut self, kind: &str, values: &[f32]) -> Result<bool, String> {
        if !matches!(kind, "none" | "calm" | "wind" | "gust" | "swirl" | "rain") {
            return Err("Unsupported motion type".into());
        }
        if values.len() != 4 || values.iter().any(|v| !v.is_finite()) {
            return Err("Expected four finite motion parameters".into());
        }
        let [base, peak, interval, duration] = [values[0], values[1], values[2], values[3]];
        if !(0.0..=1.0).contains(&base)
            || !(base..=1.0).contains(&peak)
            || !(0.0..=60.0).contains(&interval)
            || !(0.0..=30.0).contains(&duration)
            || (interval > 0.0 && duration > interval)
            || (matches!(kind, "none" | "calm") && values.iter().any(|v| *v != 0.0))
        {
            return Err("Motion parameters are outside the API contract".into());
        }
        // Unrecognized motion must not replace a previous successful profile.
        if kind == "none" {
            return Ok(false);
        }
        self.profile = Some(Profile {
            base,
            peak,
            interval,
            duration,
        });
        self.restart();
        Ok(true)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.restart();
        }
        self.enabled = enabled;
    }

    pub fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn step(&mut self, dt: f32) -> f32 {
        let mut target = 1.0;
        if self.enabled {
            if let Some(profile) = &self.profile {
                let pulse = if profile.duration > 0.0 && self.elapsed < profile.duration {
                    (std::f32::consts::PI * self.elapsed / profile.duration)
                        .sin()
                        .powi(2)
                } else {
                    0.0
                };
                target = 2.0 * (profile.base + (profile.peak - profile.base) * pulse);
                if profile.interval > 0.0 {
                    self.elapsed = (self.elapsed + dt) % profile.interval;
                } else {
                    self.elapsed = (self.elapsed + dt).min(profile.duration);
                }
            }
        }
        // Smooth changes without resetting velocities or changing the solver timestep.
        self.gain += (target - self.gain) * (1.0 - (-dt / 0.35).exp());
        self.gain
    }
}
