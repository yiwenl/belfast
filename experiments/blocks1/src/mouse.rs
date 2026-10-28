use belfast::{fluid::FluidSplat, PerspectiveCamera};
use glam::{Mat4, Vec2, Vec3};

#[derive(Default)]
pub struct MouseFlow {
    current: Option<Vec2>,
    previous: Option<Vec2>,
}

impl MouseFlow {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn move_to(&mut self, camera: &PerspectiveCamera, x: f32, y: f32, width: f32, height: f32) {
        if ![x, y, width, height].iter().all(|v| v.is_finite())
            || width <= 0.0
            || height <= 0.0
            || !(0.0..=width).contains(&x)
            || !(0.0..=height).contains(&y)
        {
            self.clear();
            return;
        }
        let inverse = Mat4::from_cols_array(&camera.view_projection_matrix()).inverse();
        let ndc = Vec2::new(x / width * 2.0 - 1.0, 1.0 - y / height * 2.0);
        let near = inverse.project_point3(ndc.extend(0.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        let ray = far - near;
        if !near.is_finite() || !ray.is_finite() || ray.z.abs() < 0.00001 {
            self.clear();
            return;
        }
        let distance = -near.z / ray.z;
        let world: Vec3 = near + ray * distance;
        // Match the particle shader's world XY → fluid UV mapping on the z=0 plane.
        let uv = world.truncate() / 12.0 + Vec2::splat(0.5);
        if distance < 0.0
            || !uv.is_finite()
            || uv.cmplt(Vec2::ZERO).any()
            || uv.cmpgt(Vec2::ONE).any()
        {
            self.clear();
            return;
        }
        self.current = Some(uv);
        self.previous.get_or_insert(uv);
    }

    pub fn has_motion(&self) -> bool {
        self.current
            .zip(self.previous)
            .is_some_and(|(current, previous)| {
                current.distance_squared(previous) >= 0.000001 * 0.000001
            })
    }

    pub fn append_splats(
        &mut self,
        strength: f32,
        radius: f32,
        max_splats: usize,
        splats: &mut Vec<FluidSplat>,
    ) {
        let (Some(current), Some(previous)) = (self.current, self.previous) else {
            return;
        };
        self.previous = Some(current);
        let delta = current - previous;
        let distance = delta.length();
        if distance < 0.000001 || strength == 0.0 || max_splats == 0 {
            return;
        }
        let count = (distance / (radius * 0.5))
            .ceil()
            .clamp(1.0, max_splats as f32) as u32;
        // Integrate displacement once per simulation step, independent of pointer event rate.
        let impulse = delta.clamp_length_max(0.2) * strength / count as f32;
        for i in 0..count {
            let position = previous.lerp(current, (i as f32 + 0.5) / count as f32);
            splats.push(FluidSplat {
                position: position.to_array(),
                velocity: impulse.to_array(),
                radius,
                density: (distance * 8.0).min(2.0) / count as f32,
            });
        }
    }
}
