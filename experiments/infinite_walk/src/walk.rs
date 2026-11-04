pub const START: [f32; 3] = [-0.3, 0.3, 4.3];
pub const END: [f32; 3] = [0.1, 0.3, -9.0];
pub const LENGTH: f32 = START[2] - END[2];
pub const DRIFT_X: f32 = END[0] - START[0];

#[derive(Default)]
pub struct Walk {
    target_phase: f64,
    remaining: f64,
    moving: bool,
}

impl Walk {
    pub fn scroll(&mut self, pixels: f32) {
        if !pixels.is_finite() {
            return;
        }
        let distance = f64::from(pixels) * 0.0005;
        self.target_phase = (self.target_phase + distance).rem_euclid(1.0);
        self.remaining += distance;
    }

    /// Returns true once, on the frame an eased walk comes to rest.
    pub fn advance(&mut self, seconds: f32) -> bool {
        if !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }
        self.remaining *= (-12.0 * f64::from(seconds.min(0.1))).exp();
        if self.remaining.abs() < 1e-9 {
            self.remaining = 0.0;
            return std::mem::take(&mut self.moving);
        }
        self.moving = true;
        false
    }

    pub fn position(&self) -> [f32; 3] {
        // Rebase each lap together with the cloud to keep GPU coordinates small indefinitely.
        let phase = (self.target_phase - self.remaining).rem_euclid(1.0) as f32;
        [
            START[0] + DRIFT_X * phase,
            START[1],
            START[2] - LENGTH * phase,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(walk: &mut Walk) {
        for _ in 0..240 {
            walk.advance(1.0 / 60.0);
        }
    }

    fn close(a: [f32; 3], b: [f32; 3]) {
        for (a, b) in a.into_iter().zip(b) {
            assert!((a - b).abs() < 0.0001, "{a} != {b}");
        }
    }

    #[test]
    fn wheel_moves_forward_smoothly_then_reverses_to_start() {
        let mut walk = Walk::default();
        walk.scroll(500.0);
        close(walk.position(), START);
        walk.advance(1.0 / 60.0);
        assert!(walk.position()[2] < 4.3 && walk.position()[2] > 0.975);
        settle(&mut walk);
        close(walk.position(), [-0.2, 0.3, 0.975]);
        walk.scroll(-500.0);
        settle(&mut walk);
        close(walk.position(), START);
    }

    #[test]
    fn forward_and_backward_cross_many_laps_without_clamping() {
        for pixels in [20_500.0, -19_500.0, 2_000_500.0, -1_999_500.0] {
            let mut walk = Walk::default();
            walk.scroll(pixels);
            settle(&mut walk);
            close(walk.position(), [-0.2, 0.3, 0.975]);
        }
    }

    #[test]
    fn damping_is_frame_rate_independent_and_ignores_invalid_input() {
        let mut a = Walk::default();
        let mut b = Walk::default();
        a.scroll(500.0);
        b.scroll(500.0);
        a.scroll(f32::NAN);
        a.scroll(f32::INFINITY);
        a.advance(f32::NAN);
        a.advance(-1.0);
        for _ in 0..30 {
            a.advance(1.0 / 30.0);
        }
        for _ in 0..120 {
            b.advance(1.0 / 120.0);
        }
        close(a.position(), b.position());
        assert!(a.position()[2] < 1.0);
    }
}
