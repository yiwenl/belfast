use glam::{Mat4, Vec3};

use crate::{BelfastError, BelfastResult};

#[derive(Clone, Debug)]
struct CameraBase {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    view: Mat4,
}

impl CameraBase {
    fn new() -> Self {
        let mut camera = Self {
            eye: Vec3::new(0.0, 0.0, 1.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            view: Mat4::IDENTITY,
        };
        camera.look_at([0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
        camera
    }

    fn look_at(&mut self, eye: [f32; 3], target: [f32; 3]) {
        self.eye = Vec3::from_array(eye);
        self.target = Vec3::from_array(target);
        self.view = Mat4::look_at_rh(self.eye, self.target, self.up);
    }

    fn target(&self) -> [f32; 3] {
        self.target.to_array()
    }
}

#[derive(Clone, Debug)]
pub struct PerspectiveCamera {
    base: CameraBase,
    fovy_radians: f32,
    aspect: f32,
    near: f32,
    far: f32,
    projection: Mat4,
}

impl PerspectiveCamera {
    pub fn new(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        let mut camera = Self {
            base: CameraBase::new(),
            fovy_radians,
            aspect,
            near,
            far,
            projection: Mat4::IDENTITY,
        };
        camera.update_projection();
        camera
    }

    pub fn look_at(&mut self, eye: [f32; 3], target: [f32; 3]) -> &mut Self {
        self.base.look_at(eye, target);
        self
    }

    pub fn set_aspect(&mut self, aspect: f32) -> BelfastResult<&mut Self> {
        if aspect <= 0.0 {
            return Err(BelfastError::InvalidCameraAspect);
        }
        self.aspect = aspect;
        self.update_projection();
        Ok(self)
    }

    pub fn projection_matrix(&self) -> [f32; 16] {
        self.projection.to_cols_array()
    }

    pub fn view_projection_matrix(&self) -> [f32; 16] {
        (self.projection * self.base.view).to_cols_array()
    }

    pub fn look_at_target(&self) -> [f32; 3] {
        self.base.target()
    }

    fn update_projection(&mut self) {
        self.projection = Mat4::perspective_rh(self.fovy_radians, self.aspect, self.near, self.far);
    }
}

#[derive(Clone, Debug)]
pub struct OrthographicCamera {
    base: CameraBase,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
    projection: Mat4,
}

impl OrthographicCamera {
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mut camera = Self {
            base: CameraBase::new(),
            left,
            right,
            bottom,
            top,
            near,
            far,
            projection: Mat4::IDENTITY,
        };
        camera.update_projection();
        camera
    }

    pub fn look_at(&mut self, eye: [f32; 3], target: [f32; 3]) -> &mut Self {
        self.base.look_at(eye, target);
        self
    }

    pub fn projection_matrix(&self) -> [f32; 16] {
        self.projection.to_cols_array()
    }

    pub fn view_projection_matrix(&self) -> [f32; 16] {
        (self.projection * self.base.view).to_cols_array()
    }

    pub fn look_at_target(&self) -> [f32; 3] {
        self.base.target()
    }

    fn update_projection(&mut self) {
        self.projection = Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        );
    }
}
