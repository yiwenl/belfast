use crate::camera_path::CameraPose;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}
impl Bounds {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    pub fn include(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }
    pub fn corners(self) -> [Vec3; 8] {
        std::array::from_fn(|i| {
            Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            )
        })
    }
}

pub struct OverviewCamera {
    pub position: Vec3,
    pub target: Vec3,
    pub view_projection: Mat4,
}
impl OverviewCamera {
    /// Rotation only: retain reconstruction coordinates while measuring the
    /// actual cloud in the fixed overview's right/up/back basis.
    pub fn orientation() -> Mat4 {
        Mat4::look_at_rh(Vec3::new(0.6, -1., -0.8).normalize(), Vec3::ZERO, -Vec3::Y)
    }
    pub fn fit(bounds: Bounds, aspect: f32) -> Self {
        let orientation = Self::orientation();
        let first = orientation.transform_vector3(bounds.min);
        let mut oriented = Bounds::new(first, first);
        for corner in bounds.corners() {
            oriented.include(orientation.transform_vector3(corner));
        }
        Self::fit_oriented(oriented, aspect)
    }
    /// Bounds are measured from actual points in view space at load time, not
    /// from the empty corners of a world-aligned box. Six percent margin.
    pub fn fit_oriented(bounds: Bounds, aspect: f32) -> Self {
        let center = (bounds.min + bounds.max) * 0.5;
        let orientation = Self::orientation();
        let target = orientation.inverse().transform_vector3(center);
        let outward = Vec3::new(0.6, -1., -0.8).normalize();
        let half_fov = 22.5_f32.to_radians();
        let tan_y = half_fov.tan();
        let tan_x = tan_y * aspect.max(0.01);
        let span = (bounds.max - bounds.min).length().max(0.1);
        let mut distance = (span * 0.1).max(0.1);
        for corner in bounds.corners() {
            let local = corner - center;
            distance =
                distance.max(local.z + 1.06 * (local.x.abs() / tan_x).max(local.y.abs() / tan_y));
        }
        let position = target + outward * distance;
        let projection = Mat4::perspective_rh(
            half_fov * 2.,
            aspect.max(0.01),
            span * 0.001,
            distance + span * 2.,
        );
        Self {
            position,
            target,
            view_projection: projection * Mat4::look_at_rh(position, target, -Vec3::Y),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrustumInstance {
    pub inverse_view_projection: [f32; 16],
    pub color: [f32; 4],
}
impl FrustumInstance {
    /// DrawCamera's inverse projection method, with WebGPU's 0..1 clip depth.
    /// A short visualization far plane keeps the full walk readable.
    pub fn new(pose: CameraPose, aspect: f32, length: f32, color: [f32; 4]) -> Self {
        let projection = Mat4::perspective_rh(pose.fov_y, aspect, length * 0.06, length);
        Self {
            inverse_view_projection: (projection * pose.view_matrix()).inverse().to_cols_array(),
            color,
        }
    }
}
