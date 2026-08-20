use glam::Vec3;

use crate::{BelfastError, BelfastResult, PerspectiveCamera};

const SNAP_EPSILON: f32 = 0.00001;
const PITCH_LIMIT: f64 = std::f64::consts::FRAC_PI_2 - 0.0001;

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
    enabled: bool,
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
            enabled: true,
            drag_mode: None,
        })
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.drag_mode = None;
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn pointer_down(
        &mut self,
        position: [f32; 2],
        button: OrbitalPointerButton,
        pan_modifier: bool,
    ) {
        if !self.enabled || !position.iter().all(|value| value.is_finite()) {
            return;
        }

        self.drag_mode = Some(
            if button == OrbitalPointerButton::Primary && !pan_modifier {
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
                    start_yaw: self.target_yaw,
                    start_pitch: self.target_pitch,
                }
            },
        );
    }

    pub fn pointer_move(&mut self, position: [f32; 2], viewport: [f32; 2]) {
        if !self.enabled
            || !position.iter().all(|value| value.is_finite())
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
                let horizontal_displacement = position[0] as f64 - start[0] as f64;
                let vertical_displacement = position[1] as f64 - start[1] as f64;
                let sensitivity = self.rotate_sensitivity as f64;
                self.target_yaw =
                    normalize_yaw(start_yaw as f64 - horizontal_displacement * sensitivity) as f32;
                self.target_pitch =
                    clamp_pitch(start_pitch as f64 + vertical_displacement * sensitivity);
            }
            Some(DragMode::Pan {
                start,
                start_center,
                start_yaw,
                start_pitch,
                ..
            }) => {
                let yaw = start_yaw as f64;
                let pitch = start_pitch as f64;
                let right = [yaw.cos(), 0.0, -yaw.sin()];
                let camera_up = [
                    -yaw.sin() * pitch.sin(),
                    pitch.cos(),
                    -yaw.cos() * pitch.sin(),
                ];
                let horizontal_displacement = position[0] as f64 - start[0] as f64;
                let vertical_displacement = position[1] as f64 - start[1] as f64;
                let scale =
                    self.target_radius as f64 * self.pan_sensitivity as f64 / viewport[1] as f64;
                let start_center = start_center.to_array().map(f64::from);
                let candidate = std::array::from_fn(|index| {
                    start_center[index] - right[index] * horizontal_displacement * scale
                        + camera_up[index] * vertical_displacement * scale
                });

                if let Some(candidate) = vec3_from_f64(candidate) {
                    self.target_center = candidate;
                }
            }
            None => {}
        }
    }

    pub fn pointer_up(&mut self, button: OrbitalPointerButton) {
        if !self.enabled {
            return;
        }
        if self
            .drag_mode
            .as_ref()
            .is_some_and(|drag_mode| drag_mode.button() == button)
        {
            self.drag_mode = None;
        }
    }

    pub fn scroll(&mut self, delta: f32) {
        if !self.enabled || !delta.is_finite() {
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

        let center = self.center.to_array();
        let target_center = self.target_center.to_array();
        self.center = Vec3::from_array(std::array::from_fn(|index| {
            interpolate(center[index], target_center[index], interpolation)
        }));
        self.center.x = snap(self.center.x, self.target_center.x);
        self.center.y = snap(self.center.y, self.target_center.y);
        self.center.z = snap(self.center.z, self.target_center.z);
        self.radius = snap(
            interpolate(self.radius, self.target_radius, interpolation),
            self.target_radius,
        );
        self.yaw = interpolate_yaw(self.yaw, self.target_yaw, interpolation);
        self.pitch = snap(
            interpolate(self.pitch, self.target_pitch, interpolation),
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

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        if let Some(yaw) = finite_yaw(yaw) {
            self.cancel_drag();
            self.target_yaw = yaw;
        }
    }

    pub fn snap_yaw(&mut self, yaw: f32) {
        if let Some(yaw) = finite_yaw(yaw) {
            self.cancel_drag();
            self.yaw = yaw;
            self.target_yaw = yaw;
        }
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        if let Some(pitch) = finite_pitch(pitch) {
            self.cancel_drag();
            self.target_pitch = pitch;
        }
    }

    pub fn snap_pitch(&mut self, pitch: f32) {
        if let Some(pitch) = finite_pitch(pitch) {
            self.cancel_drag();
            self.pitch = pitch;
            self.target_pitch = pitch;
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        if let Some(radius) = self.clamped_radius(radius) {
            self.cancel_drag();
            self.target_radius = radius;
        }
    }

    pub fn snap_radius(&mut self, radius: f32) {
        if let Some(radius) = self.clamped_radius(radius) {
            self.cancel_drag();
            self.radius = radius;
            self.target_radius = radius;
        }
    }

    fn cancel_drag(&mut self) {
        self.drag_mode = None;
    }

    fn clamped_radius(&self, radius: f32) -> Option<f32> {
        radius
            .is_finite()
            .then_some(radius.clamp(self.min_radius, self.max_radius))
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
    if !options.rotate_sensitivity.is_finite() || options.rotate_sensitivity < 0.0 {
        return Err(BelfastError::InvalidOrbitalControlOption(
            "rotate_sensitivity",
        ));
    }
    if !options.zoom_sensitivity.is_finite() || options.zoom_sensitivity < 0.0 {
        return Err(BelfastError::InvalidOrbitalControlOption(
            "zoom_sensitivity",
        ));
    }
    if !options.pan_sensitivity.is_finite() || options.pan_sensitivity < 0.0 {
        return Err(BelfastError::InvalidOrbitalControlOption("pan_sensitivity"));
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

fn normalize_yaw(yaw: f64) -> f64 {
    (yaw + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn clamp_pitch(pitch: f64) -> f32 {
    pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT) as f32
}

fn finite_yaw(yaw: f32) -> Option<f32> {
    yaw.is_finite().then_some(normalize_yaw(yaw as f64) as f32)
}

fn finite_pitch(pitch: f32) -> Option<f32> {
    pitch.is_finite().then_some(clamp_pitch(pitch as f64))
}

fn vec3_from_f64(values: [f64; 3]) -> Option<Vec3> {
    if values.iter().all(|value| f32_from_f64(*value).is_some()) {
        Some(Vec3::from_array(values.map(|value| value as f32)))
    } else {
        None
    }
}

fn f32_from_f64(value: f64) -> Option<f32> {
    (value.is_finite() && value.abs() <= f32::MAX as f64).then_some(value as f32)
}

fn interpolate(value: f32, target: f32, interpolation: f32) -> f32 {
    let candidate = value as f64 + (target as f64 - value as f64) * interpolation as f64;
    f32_from_f64(candidate).unwrap_or(value)
}

fn interpolate_yaw(value: f32, target: f32, interpolation: f32) -> f32 {
    let delta = normalize_yaw(target as f64 - value as f64);
    if delta.abs() <= SNAP_EPSILON as f64 {
        target
    } else {
        normalize_yaw(value as f64 + delta * interpolation as f64) as f32
    }
}

fn snap(value: f32, target: f32) -> f32 {
    if (value as f64 - target as f64).abs() <= SNAP_EPSILON as f64 {
        target
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::interpolate;

    #[test]
    fn interpolation_between_opposite_f32_extremes_stays_finite() {
        let value = interpolate(f32::MAX, -f32::MAX, 0.5);

        assert!(value.is_finite());
        assert_eq!(value, 0.0);
    }
}
