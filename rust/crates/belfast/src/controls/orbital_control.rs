use glam::Vec3;

use crate::{BelfastError, BelfastResult, PerspectiveCamera};

const SNAP_EPSILON: f32 = 0.00001;

#[derive(Clone, Copy, Debug)]
enum DragMode {
    Rotate {
        button: OrbitalPointerButton,
        start: [f32; 2],
        start_yaw: f32,
        start_pitch: f32,
    },
    Pan {
        button: OrbitalPointerButton,
        start: [f32; 2],
        start_center: Vec3,
        start_radius: f32,
        start_yaw: f32,
        start_pitch: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbitalPointerButton {
    Primary,
    Middle,
}

#[derive(Clone, Copy, Debug)]
pub struct OrbitalControlOptions {
    pub center: [f32; 3],
    pub radius: f32,
    pub min_radius: f32,
    pub max_radius: f32,
    pub rotate_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub damping: f32,
}

impl Default for OrbitalControlOptions {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0, 0.0],
            radius: 10.0,
            min_radius: 0.0001,
            max_radius: f32::MAX,
            rotate_sensitivity: 0.01,
            zoom_sensitivity: 0.002,
            pan_sensitivity: 2.0,
            damping: 12.0,
        }
    }
}

#[derive(Debug)]
pub struct OrbitalControl {
    center: Vec3,
    target_center: Vec3,
    radius: f32,
    target_radius: f32,
    yaw: f32,
    target_yaw: f32,
    pitch: f32,
    target_pitch: f32,
    min_radius: f32,
    max_radius: f32,
    rotate_sensitivity: f32,
    zoom_sensitivity: f32,
    pan_sensitivity: f32,
    damping: f32,
    drag_mode: Option<DragMode>,
}

impl OrbitalControl {
    pub fn new(options: OrbitalControlOptions) -> BelfastResult<Self> {
        validate_options(options)?;

        let center = Vec3::from_array(options.center);
        Ok(Self {
            center,
            target_center: center,
            radius: options.radius,
            target_radius: options.radius,
            yaw: 0.0,
            target_yaw: 0.0,
            pitch: 0.0,
            target_pitch: 0.0,
            min_radius: options.min_radius,
            max_radius: options.max_radius,
            rotate_sensitivity: options.rotate_sensitivity,
            zoom_sensitivity: options.zoom_sensitivity,
            pan_sensitivity: options.pan_sensitivity,
            damping: options.damping,
            drag_mode: None,
        })
    }

    pub fn pointer_down(
        &mut self,
        position: [f32; 2],
        button: OrbitalPointerButton,
        shift_key: bool,
    ) {
        if !position.iter().all(|value| value.is_finite()) {
            return;
        }

        self.drag_mode = Some(if button == OrbitalPointerButton::Primary && !shift_key {
            DragMode::Rotate {
                button,
                start: position,
                start_yaw: self.target_yaw,
                start_pitch: self.target_pitch,
            }
        } else {
            DragMode::Pan {
                button,
                start: position,
                start_center: self.target_center,
                start_radius: self.target_radius,
                start_yaw: self.target_yaw,
                start_pitch: self.target_pitch,
            }
        });
    }

    pub fn pointer_move(&mut self, position: [f32; 2], viewport: [f32; 2]) {
        if !position.iter().all(|value| value.is_finite())
            || !viewport.iter().all(|value| value.is_finite())
            || viewport[0] <= 0.0
            || viewport[1] <= 0.0
        {
            return;
        }

        match self.drag_mode {
            Some(DragMode::Rotate {
                start,
                start_yaw,
                start_pitch,
                ..
            }) => {
                self.target_yaw = start_yaw + (position[0] - start[0]) * self.rotate_sensitivity;
                self.target_pitch =
                    (start_pitch + (position[1] - start[1]) * self.rotate_sensitivity).clamp(
                        -std::f32::consts::FRAC_PI_2 + 0.0001,
                        std::f32::consts::FRAC_PI_2 - 0.0001,
                    );
            }
            Some(DragMode::Pan {
                start,
                start_center,
                start_radius,
                start_yaw,
                start_pitch,
                ..
            }) => {
                let eye = eye_from_pose(start_center, start_radius, start_yaw, start_pitch);
                let forward = (start_center - eye).normalize();
                let right = forward.cross(Vec3::Y).normalize();
                let camera_up = right.cross(forward).normalize();
                let scale = self.target_radius * self.pan_sensitivity / viewport[1];
                self.target_center = start_center - right * (position[0] - start[0]) * scale
                    + camera_up * (position[1] - start[1]) * scale;
            }
            None => {}
        }
    }

    pub fn pointer_up(&mut self, button: OrbitalPointerButton) {
        if self
            .drag_mode
            .as_ref()
            .is_some_and(|drag_mode| drag_mode.button() == button)
        {
            self.drag_mode = None;
        }
    }

    pub fn scroll(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }

        let exponent = (delta * self.zoom_sensitivity).clamp(-80.0, 80.0);
        self.target_radius =
            (self.target_radius * exponent.exp()).clamp(self.min_radius, self.max_radius);
    }

    pub fn update(&mut self, delta_seconds: f32, camera: &mut PerspectiveCamera) -> bool {
        let interpolation = response(self.damping, delta_seconds);
        let previous_center = self.center;
        let previous_radius = self.radius;
        let previous_yaw = self.yaw;
        let previous_pitch = self.pitch;

        self.center = self.center.lerp(self.target_center, interpolation);
        self.center.x = snap(self.center.x, self.target_center.x);
        self.center.y = snap(self.center.y, self.target_center.y);
        self.center.z = snap(self.center.z, self.target_center.z);
        self.radius = snap(
            self.radius + (self.target_radius - self.radius) * interpolation,
            self.target_radius,
        );
        self.yaw = snap(
            self.yaw + (self.target_yaw - self.yaw) * interpolation,
            self.target_yaw,
        );
        self.pitch = snap(
            self.pitch + (self.target_pitch - self.pitch) * interpolation,
            self.target_pitch,
        );

        camera.look_at(self.eye(), self.center());

        self.center != previous_center
            || self.radius != previous_radius
            || self.yaw != previous_yaw
            || self.pitch != previous_pitch
    }

    pub fn center(&self) -> [f32; 3] {
        self.center.to_array()
    }

    pub fn eye(&self) -> [f32; 3] {
        eye_from_pose(self.center, self.radius, self.yaw, self.pitch).to_array()
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}

impl DragMode {
    fn button(&self) -> OrbitalPointerButton {
        match self {
            Self::Rotate { button, .. } | Self::Pan { button, .. } => *button,
        }
    }
}

fn validate_options(options: OrbitalControlOptions) -> BelfastResult<()> {
    if !options.center.iter().all(|value| value.is_finite()) {
        return Err(BelfastError::InvalidOrbitalControlOption("center"));
    }
    if !options.min_radius.is_finite() || options.min_radius <= 0.0 {
        return Err(BelfastError::InvalidOrbitalControlOption("min_radius"));
    }
    if !options.max_radius.is_finite() || options.max_radius < options.min_radius {
        return Err(BelfastError::InvalidOrbitalControlOption("max_radius"));
    }
    if !options.radius.is_finite()
        || options.radius < options.min_radius
        || options.radius > options.max_radius
    {
        return Err(BelfastError::InvalidOrbitalControlOption("radius"));
    }
    if !options.rotate_sensitivity.is_finite() {
        return Err(BelfastError::InvalidOrbitalControlOption(
            "rotate_sensitivity",
        ));
    }
    if !options.zoom_sensitivity.is_finite() {
        return Err(BelfastError::InvalidOrbitalControlOption(
            "zoom_sensitivity",
        ));
    }
    if !options.pan_sensitivity.is_finite() {
        return Err(BelfastError::InvalidOrbitalControlOption("pan_sensitivity"));
    }
    if options.rotate_sensitivity < 0.0
        || options.zoom_sensitivity < 0.0
        || options.pan_sensitivity < 0.0
    {
        return Err(BelfastError::InvalidOrbitalControlOption("sensitivity"));
    }
    if !options.damping.is_finite() || options.damping < 0.0 {
        return Err(BelfastError::InvalidOrbitalControlOption("damping"));
    }
    Ok(())
}

fn eye_from_pose(center: Vec3, radius: f32, yaw: f32, pitch: f32) -> Vec3 {
    let horizontal_radius = pitch.cos() * radius;
    let vertical_radius = pitch.sin().clamp(-1.0 + f32::EPSILON, 1.0 - f32::EPSILON) * radius;
    center
        + Vec3::new(
            yaw.sin() * horizontal_radius,
            vertical_radius,
            yaw.cos() * horizontal_radius,
        )
}

fn response(damping: f32, delta_seconds: f32) -> f32 {
    if damping == 0.0 {
        1.0
    } else {
        1.0 - (-damping * delta_seconds.max(0.0)).exp()
    }
}

fn snap(value: f32, target: f32) -> f32 {
    if (value - target).abs() <= SNAP_EPSILON {
        target
    } else {
        value
    }
}
