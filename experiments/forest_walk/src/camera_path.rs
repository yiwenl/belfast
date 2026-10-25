use glam::{Mat3, Mat4, Quat, Vec3};
use serde::Deserialize;

#[derive(Clone, Copy, Debug)]
pub struct CameraPose {
    pub position: Vec3,
    /// Camera-local right/up/back axes in the original reconstruction world.
    pub rotation: Quat,
    pub fov_y: f32,
    pub aspect: f32,
}

impl CameraPose {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position).inverse()
    }
}

#[derive(Deserialize)]
struct Recording {
    #[serde(default)]
    cameras: Vec<Camera>,
    #[serde(default)]
    undistorted_cameras: Vec<Camera>,
    frames: Vec<Frame>,
}
#[derive(Deserialize)]
struct Camera {
    camera_id: u32,
    width: u32,
    height: u32,
    named_parameters: Intrinsics,
}
#[derive(Deserialize)]
struct Intrinsics {
    fy: Option<f32>,
    f: Option<f32>,
}
#[derive(Deserialize)]
struct Frame {
    camera_id: u32,
    timestamp_seconds: f64,
    camera_to_world: [[f32; 4]; 4],
}

#[derive(Debug)]
pub struct CameraTrack {
    poses: Vec<CameraPose>,
    timestamps: Vec<f64>,
}

impl CameraTrack {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut recording: Recording =
            serde_json::from_str(json).map_err(|e| format!("Invalid camera JSON: {e}"))?;
        if recording.frames.is_empty() {
            return Err("Camera recording contains no frames".into());
        }
        if recording
            .frames
            .iter()
            .any(|f| !f.timestamp_seconds.is_finite() || f.timestamp_seconds < 0.)
        {
            return Err("Camera timestamps must be finite and non-negative".into());
        }
        recording
            .frames
            .sort_by(|a, b| a.timestamp_seconds.total_cmp(&b.timestamp_seconds));
        if recording
            .frames
            .windows(2)
            .any(|pair| pair[0].timestamp_seconds == pair[1].timestamp_seconds)
        {
            return Err("Camera timestamps must be unique".into());
        }
        let mut poses = Vec::with_capacity(recording.frames.len());
        for frame in &recording.frames {
            let camera = recording
                .undistorted_cameras
                .iter()
                .chain(recording.cameras.iter())
                .find(|c| c.camera_id == frame.camera_id)
                .ok_or("Missing intrinsics for a recorded camera")?;
            let fy = camera
                .named_parameters
                .fy
                .or(camera.named_parameters.f)
                .ok_or("Missing camera focal length")?;
            if !fy.is_finite() || fy <= 0. || camera.width == 0 || camera.height == 0 {
                return Err("Invalid camera intrinsics".into());
            }
            let fov_y = 2. * (camera.height as f32 / (2. * fy)).atan();
            let m = &frame.camera_to_world;
            if !m.iter().flatten().all(|v| v.is_finite())
                || m[3]
                    .iter()
                    .zip([0., 0., 0., 1.])
                    .any(|(a, b)| (a - b).abs() > 1e-4)
            {
                return Err("Invalid camera transform".into());
            }
            let right = Vec3::new(m[0][0], m[1][0], m[2][0]);
            let down = Vec3::new(m[0][1], m[1][1], m[2][1]);
            let forward = Vec3::new(m[0][2], m[1][2], m[2][2]);
            if [right, down, forward]
                .iter()
                .any(|v| (v.length_squared() - 1.).abs() > 0.005)
                || right.dot(down).abs() > 0.005
                || right.dot(forward).abs() > 0.005
                || down.dot(forward).abs() > 0.005
                || (right.cross(down).dot(forward) - 1.).abs() > 0.005
            {
                return Err("Camera transform must contain a rigid rotation".into());
            }
            // JSON matrices are row-major; columns are OpenCV right/down/forward.
            // The renderer looks down local -Z with local +Y up. Do not flip the world/PLY.
            let rotation = Quat::from_mat3(&Mat3::from_cols(right, -down, -forward)).normalize();
            poses.push(CameraPose {
                position: Vec3::new(m[0][3], m[1][3], m[2][3]),
                rotation,
                fov_y,
                aspect: camera.width as f32 / camera.height as f32,
            });
        }
        let timestamps = recording
            .frames
            .iter()
            .map(|frame| frame.timestamp_seconds)
            .collect();
        Ok(Self { poses, timestamps })
    }

    pub fn len(&self) -> usize {
        self.poses.len()
    }
    pub fn is_empty(&self) -> bool {
        self.poses.is_empty()
    }

    pub fn sample_time(&self, cursor: f32) -> f64 {
        let cursor = if cursor.is_finite() {
            cursor.clamp(0., (self.len() - 1) as f32)
        } else {
            0.
        };
        let index = cursor.floor() as usize;
        let next = (index + 1).min(self.len() - 1);
        let t = f64::from(cursor - index as f32);
        self.timestamps[index] + (self.timestamps[next] - self.timestamps[index]) * t
    }

    pub fn sample(&self, cursor: f32) -> CameraPose {
        let cursor = if cursor.is_finite() {
            cursor.clamp(0., (self.len() - 1) as f32)
        } else {
            0.
        };
        let index = cursor.floor() as usize;
        let next = (index + 1).min(self.len() - 1);
        let t = cursor - index as f32;
        let a = self.poses[index];
        let b = self.poses[next];
        CameraPose {
            position: a.position.lerp(b.position, t),
            rotation: a.rotation.slerp(b.rotation, t).normalize(),
            fov_y: a.fov_y + (b.fov_y - a.fov_y) * t,
            aspect: a.aspect + (b.aspect - a.aspect) * t,
        }
    }
}

pub struct CameraNavigator {
    track: CameraTrack,
    cursor: f32,
    target: usize,
    initial_rotation: Quat,
    yaw: f32,
    pitch: f32,
    fov_y: f32,
    aspect: f32,
}

impl CameraNavigator {
    pub fn new(track: CameraTrack) -> Self {
        let initial = track.sample(0.);
        Self {
            track,
            cursor: 0.,
            target: 0,
            initial_rotation: initial.rotation,
            yaw: 0.,
            pitch: 0.,
            fov_y: initial.fov_y,
            aspect: initial.aspect,
        }
    }
    pub fn step(&mut self, direction: i32) {
        self.target = self
            .target
            .saturating_add_signed(direction.signum() as isize)
            .min(self.len() - 1);
    }
    /// Rotate at the current position, relative to the first camera's upright frame.
    pub fn look_around(&mut self, yaw_delta: f32, pitch_delta: f32) {
        if !yaw_delta.is_finite() || !pitch_delta.is_finite() {
            return;
        }
        use std::f32::consts::{FRAC_PI_2, PI, TAU};
        self.yaw = (self.yaw + yaw_delta.rem_euclid(TAU) + PI).rem_euclid(TAU) - PI;
        let limit = FRAC_PI_2 - 0.01;
        self.pitch = (self.pitch + pitch_delta).clamp(-limit, limit);
    }

    pub fn advance(&mut self, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0. {
            return;
        }
        let remaining = self.target as f32 - self.cursor;
        self.cursor += remaining * (1. - (-seconds.min(0.1) / 0.14).exp());
        if (self.target as f32 - self.cursor).abs() < 0.0001 {
            self.cursor = self.target as f32;
        }
    }
    pub fn pose(&self) -> CameraPose {
        CameraPose {
            position: self.track.sample(self.cursor).position,
            rotation: (self.initial_rotation
                * Quat::from_rotation_y(self.yaw)
                * Quat::from_rotation_x(self.pitch))
            .normalize(),
            fov_y: self.fov_y,
            aspect: self.aspect,
        }
    }
    pub fn timestamp_seconds(&self) -> f64 {
        self.track.sample_time(self.cursor)
    }

    pub fn index(&self) -> usize {
        self.target
    }
    pub fn len(&self) -> usize {
        self.track.len()
    }
    pub fn is_empty(&self) -> bool {
        self.track.is_empty()
    }
}
