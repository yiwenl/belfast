use std::sync::{Arc, Mutex};

use belfast::Device;
use wasm_bindgen::prelude::*;

use crate::particles::ParticleSettings;
use crate::scene::Scene;

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// A complete Rust-owned scene. JS supplies browser events and frame timestamps only.
#[wasm_bindgen]
pub struct ExperimentApp {
    scene: Scene,
    surface: wgpu::Surface<'static>,
    instance: wgpu::Instance,
    device: Device,
    canvas: web_sys::HtmlCanvasElement,
    config: wgpu::SurfaceConfiguration,
    drawable: bool,
    last_frame_ms: Option<f64>,
    gpu_error: Arc<Mutex<Option<String>>>,
}

#[wasm_bindgen]
impl ExperimentApp {
    pub async fn create(canvas: web_sys::HtmlCanvasElement) -> Result<ExperimentApp, JsValue> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        let (gpu, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ExperimentDevice"),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        let gpu_error = Arc::new(Mutex::new(None));
        let pending = gpu_error.clone();
        gpu.on_uncaptured_error(Arc::new(move |error| {
            // Preserve the cause instead of replacing it with cascading invalid-command errors.
            pending
                .lock()
                .unwrap()
                .get_or_insert_with(|| error.to_string());
        }));
        let pending = gpu_error.clone();
        gpu.set_device_lost_callback(move |reason, message| {
            *pending.lock().unwrap() = Some(format!("WebGPU device lost ({reason:?}): {message}"));
        });
        let capabilities = surface.get_capabilities(&adapter);
        if capabilities.formats.is_empty() && capabilities.format_capabilities.is_empty() {
            return Err(js_error("Canvas has no supported WebGPU surface format"));
        }
        let color = belfast::pick_surface_color(&capabilities, false);
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| js_error("Canvas has no supported alpha mode"))?;
        let device = Device::from_wgpu(gpu, queue, color.format, Some(color.hdr));
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: color.format,
            color_space: color.color_space,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };
        let scene = Scene::new(&device).map_err(js_error)?;
        Ok(Self {
            scene,
            surface,
            instance,
            device,
            canvas,
            config,
            drawable: false,
            last_frame_ms: None,
            gpu_error,
        })
    }

    /// CSS dimensions; pointer coordinates and viewport dimensions also use CSS pixels.
    pub fn resize(&mut self, width: u32, height: u32, dpr: f64) -> Result<(), JsValue> {
        if width == 0 || height == 0 {
            self.drawable = false;
            return Ok(());
        }
        if !dpr.is_finite() || dpr <= 0.0 {
            return Err(js_error("Device pixel ratio must be positive and finite"));
        }
        let limit = self.device.gpu().limits().max_texture_dimension_2d as f64;
        let scale = dpr.min(limit / f64::from(width.max(height)));
        let width = (f64::from(width) * scale).round().max(1.0) as u32;
        let height = (f64::from(height) * scale).round().max(1.0) as u32;
        if !self.drawable || self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.surface.configure(self.device.gpu(), &self.config);
            self.scene
                .camera
                .set_aspect(width as f32 / height as f32)
                .map_err(js_error)?;
        }
        self.scene.resize(&self.device, width, height, scale as f32);
        self.drawable = true;
        Ok(())
    }

    /// Returns true when the canvas contains a freshly rendered frame for capture.
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<bool, JsValue> {
        if !timestamp_ms.is_finite() || timestamp_ms < 0.0 {
            return Err(js_error(
                "Frame timestamp must be finite, non-negative milliseconds",
            ));
        }
        if let Some(error) = self.gpu_error.lock().unwrap().take() {
            return Err(js_error(error));
        }
        let previous = self.last_frame_ms.unwrap_or(timestamp_ms);
        let delta_seconds = ((timestamp_ms - previous) * 0.001).max(0.0) as f32;
        self.last_frame_ms = Some(timestamp_ms.max(previous));
        self.scene
            .update(&self.device, delta_seconds.min(0.1), timestamp_ms);
        if !self.drawable {
            return Ok(false);
        }
        self.scene
            .simulate(&self.device, delta_seconds)
            .map_err(js_error)?;
        let (frame, reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(self.device.gpu(), &self.config);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(wgpu::SurfaceTarget::Canvas(self.canvas.clone()))
                    .map_err(js_error)?;
                self.surface.configure(self.device.gpu(), &self.config);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(js_error("WebGPU surface validation failed"))
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.scene.render(&self.device, &view);
        self.device.queue().present(frame);
        if reconfigure {
            self.surface.configure(self.device.gpu(), &self.config);
        }
        Ok(!reconfigure)
    }

    pub fn scroll(&mut self, delta: f32) {
        self.scene.walk.scroll(delta);
    }

    #[wasm_bindgen(js_name = toggleAutoBackward)]
    pub fn toggle_auto_backward(&mut self) {
        self.scene.walk.toggle_auto_backward();
    }

    #[wasm_bindgen(js_name = setPointSize)]
    pub fn set_point_size(&mut self, size: f32) -> Result<(), JsValue> {
        self.scene.set_point_size(size).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setCameraY)]
    pub fn set_camera_y(&mut self, y: f32) -> Result<(), JsValue> {
        self.scene.set_camera_y(y).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setCopyOffset)]
    pub fn set_copy_offset(&mut self, offset: f32) -> Result<(), JsValue> {
        self.scene.set_copy_offset(offset).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setCameraPitch)]
    pub fn set_camera_pitch(&mut self, degrees: f32) -> Result<(), JsValue> {
        self.scene.set_camera_pitch(degrees).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setFov)]
    pub fn set_fov(&mut self, degrees: f32) -> Result<(), JsValue> {
        if !degrees.is_finite() || !(10.0..=120.0).contains(&degrees) {
            return Err(js_error("FOV must be between 10 and 120 degrees"));
        }
        self.scene
            .camera
            .set_fovy_radians(degrees.to_radians())
            .map(|_| ())
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setWhiteParticles)]
    pub fn set_white_particles(&mut self, enabled: bool) {
        self.scene.set_white_particles(enabled);
    }

    #[wasm_bindgen(js_name = setShadowStrength)]
    pub fn set_shadow_strength(&mut self, strength: f32) -> Result<(), JsValue> {
        self.scene.set_shadow_strength(strength).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setShowShadowMap)]
    pub fn set_show_shadow_map(&mut self, enabled: bool) {
        self.scene.set_show_shadow_map(enabled);
    }

    #[wasm_bindgen(js_name = setFogDensity)]
    pub fn set_fog_density(&mut self, density: f32) -> Result<(), JsValue> {
        self.scene.set_fog_density(density).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setFogHalfHeight)]
    pub fn set_fog_half_height(&mut self, height: f32) -> Result<(), JsValue> {
        self.scene.set_fog_half_height(height).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setLight)]
    pub fn set_light(
        &mut self,
        forward: f32,
        height: f32,
        intensity: f32,
        range: f32,
        radius: f32,
    ) -> Result<(), JsValue> {
        self.scene
            .set_light(forward, height, intensity, range, radius)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setParticles)]
    pub fn set_particles(
        &mut self,
        visible: bool,
        animate: bool,
        size: f32,
        lifetime: f32,
        life_variation: f32,
        lift: f32,
        drag: f32,
        ground_y: f32,
        count: Option<u32>,
        outside_decay: Option<f32>,
        force_multiplier: Option<f32>,
        gravity: Option<f32>,
    ) -> Result<(), JsValue> {
        let settings = ParticleSettings {
            count: count.unwrap_or(self.scene.particle_settings.count),
            visible,
            animate,
            size,
            lifetime,
            life_variation,
            outside_decay: outside_decay.unwrap_or(self.scene.particle_settings.outside_decay),
            force_multiplier: force_multiplier
                .unwrap_or(self.scene.particle_settings.force_multiplier),
            lift,
            drag,
            gravity: gravity.unwrap_or(self.scene.particle_settings.gravity),
            ground_y,
        };
        settings.validate().map_err(js_error)?;
        self.scene.particle_settings = settings;
        Ok(())
    }

    #[wasm_bindgen(js_name = setFilament)]
    pub fn set_filament(
        &mut self,
        updraft: f32,
        drift: f32,
        radius: f32,
        core: f32,
        strength: f32,
        circulation: f32,
        lifetime: f32,
        rings: Option<u32>,
        auto_reset: Option<bool>,
        reset_interval: Option<f32>,
    ) -> Result<(), JsValue> {
        let rings = rings.unwrap_or(self.scene.filament_rings);
        let reset_interval = reset_interval.unwrap_or(self.scene.filament_reset_interval);
        if !(1..=crate::particles::MAX_RINGS).contains(&rings) {
            return Err(js_error("Invalid filament ring count"));
        }
        for (value, min, max) in [
            (updraft, 0.0, 1.5),
            (drift, 0.0, 0.5),
            (radius, 0.05, 2.0),
            (core, 0.005, 0.3),
            (strength, 0.0, 5.0),
            (circulation, 0.0, 0.2),
            (lifetime, 1.0, 20.0),
            (reset_interval, 0.0, 20.0),
        ] {
            if !(min..=max).contains(&value) {
                return Err(js_error("Invalid filament settings"));
            }
        }
        let settings = belfast::filament::FilamentSettings {
            origin: [0.0; 3],
            ring_radius: radius,
            core_radius: core,
            updraft,
            xz_drift: drift,
            strength,
            circulation,
            lifetime,
        };
        settings.validate().map_err(js_error)?;
        self.scene.filament_settings = settings;
        self.scene.filament_rings = rings;
        self.scene.filament_auto_reset = auto_reset.unwrap_or(self.scene.filament_auto_reset);
        self.scene.filament_reset_interval = reset_interval;
        Ok(())
    }

    #[wasm_bindgen(js_name = setFilamentDebugVisible)]
    pub fn set_filament_debug_visible(&mut self, visible: bool) {
        self.scene.show_filament_debug = visible;
    }

    #[wasm_bindgen(js_name = loadPointCloud)]
    pub fn load_point_cloud(&mut self, bytes: &[u8]) -> Result<u32, JsValue> {
        self.scene
            .load_point_cloud(&self.device, bytes)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setSkeleton)]
    pub fn set_skeleton(&mut self, points: &[f32], timestamp_ms: f64) -> Result<(), JsValue> {
        self.scene
            .skeleton
            .set(points, timestamp_ms)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearSkeleton)]
    pub fn clear_skeleton(&mut self) {
        self.scene.skeleton.clear();
    }
}
