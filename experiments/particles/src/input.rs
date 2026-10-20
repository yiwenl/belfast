pub const LANDMARK_COUNT: usize = 33;
pub const SKELETON_FLOATS: usize = LANDMARK_COUNT * 4;
pub const TRACKING_TIMEOUT_MS: f64 = 250.0;

#[derive(Default)]
pub struct SkeletonInput {
    frame: Option<([f32; SKELETON_FLOATS], f64)>,
}

impl SkeletonInput {
    pub fn set(&mut self, points: &[f32], timestamp_ms: f64) -> Result<(), &'static str> {
        if points.len() != SKELETON_FLOATS {
            return Err("Expected 33 landmarks packed as x, y, z, confidence (132 floats)");
        }
        if !timestamp_ms.is_finite() || timestamp_ms < 0.0 {
            return Err("Skeleton timestamp must be finite, non-negative milliseconds");
        }
        if self
            .frame
            .as_ref()
            .is_some_and(|(_, previous)| timestamp_ms < *previous)
        {
            return Err("Skeleton frames must arrive in timestamp order");
        }
        if !points.iter().all(|value| value.is_finite())
            || points
                .chunks_exact(4)
                .any(|point| !(0.0..=1.0).contains(&point[3]))
        {
            return Err("Skeleton coordinates must be finite and confidence must be in [0, 1]");
        }
        let mut owned = [0.0; SKELETON_FLOATS];
        owned.copy_from_slice(points);
        self.frame = Some((owned, timestamp_ms));
        Ok(())
    }

    pub fn latest(&self, now_ms: f64) -> Option<&[f32]> {
        let (points, timestamp) = self.frame.as_ref()?;
        let age = now_ms - timestamp;
        (0.0..=TRACKING_TIMEOUT_MS)
            .contains(&age)
            .then_some(points.as_slice())
    }

    pub fn clear(&mut self) {
        self.frame = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose() -> [f32; 132] {
        let mut points = [0.0; 132];
        points[0..4].copy_from_slice(&[1.0, 2.0, -3.0, 0.9]);
        points
    }

    #[test]
    fn stores_an_owned_pose_and_expires_stale_tracking() {
        let mut input = SkeletonInput::default();
        let mut points = pose();
        input.set(&points, 1000.0).unwrap();
        points[0] = 99.0;
        assert_eq!(points[0], 99.0);
        assert_eq!(&input.latest(1100.0).unwrap()[..4], &[1.0, 2.0, -3.0, 0.9]);
        assert!(input.latest(1251.0).is_none());
    }

    #[test]
    fn invalid_pose_does_not_replace_last_valid_input() {
        let mut input = SkeletonInput::default();
        input.set(&pose(), 1000.0).unwrap();
        assert!(input.set(&[0.0; 131], 1001.0).is_err());
        let mut points = pose();
        points[0] = f32::NAN;
        assert!(input.set(&points, 1001.0).is_err());
        points[0] = 0.0;
        points[3] = 1.1;
        assert!(input.set(&points, 1001.0).is_err());
        assert!(input.set(&pose(), f64::NAN).is_err());
        assert!(input.set(&pose(), -1.0).is_err());
        assert_eq!(input.latest(1100.0).unwrap()[0], 1.0);
    }

    #[test]
    fn rejects_out_of_order_frames_and_clears_lost_tracking() {
        let mut input = SkeletonInput::default();
        input.set(&pose(), 1000.0).unwrap();
        assert!(input.set(&pose(), 999.0).is_err());
        input.clear();
        assert!(input.latest(1000.0).is_none());
        input.set(&pose(), 1001.0).unwrap();
        assert!(input.latest(1001.0).is_some());
    }
}
