use std::sync::{Arc, Mutex};

use belfast::Device;
use wasm_bindgen::prelude::*;

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
            *pending.lock().unwrap() = Some(error.to_string());
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
        }
        self.scene.resize(&self.device, width, height, scale as f32);
        self.drawable = true;
        Ok(())
    }

    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        if !timestamp_ms.is_finite() || timestamp_ms < 0.0 {
            return Err(js_error(
                "Frame timestamp must be finite, non-negative milliseconds",
            ));
        }
        if let Some(error) = self.gpu_error.lock().unwrap().take() {
            return Err(js_error(error));
        }
        let previous = self.last_frame_ms.unwrap_or(timestamp_ms);
        let delta_seconds = ((timestamp_ms - previous) * 0.001).clamp(0.0, 0.1) as f32;
        self.last_frame_ms = Some(timestamp_ms.max(previous));
        self.scene.update(&self.device, delta_seconds, timestamp_ms);
        if !self.drawable {
            return Ok(());
        }
        let (frame, reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(self.device.gpu(), &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(wgpu::SurfaceTarget::Canvas(self.canvas.clone()))
                    .map_err(js_error)?;
                self.surface.configure(self.device.gpu(), &self.config);
                return Ok(());
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
        Ok(())
    }

    #[wasm_bindgen(js_name = loadPointCloud)]
    pub fn load_point_cloud(&mut self, bytes: &[u8]) -> Result<u32, JsValue> {
        self.scene
            .load_point_cloud(&self.device, bytes)
            .map_err(js_error)
    }
    #[wasm_bindgen(js_name = loadCameras)]
    pub fn load_cameras(&mut self, json: &str) -> Result<u32, JsValue> {
        self.scene.load_cameras(json).map_err(js_error)
    }
    #[wasm_bindgen(js_name = stepCamera)]
    pub fn step_camera(&mut self, direction: i32) {
        if let Some(navigation) = &mut self.scene.navigation {
            navigation.step(direction);
        }
    }
    #[wasm_bindgen(js_name = lookAround)]
    pub fn look_around(&mut self, yaw_delta: f32, pitch_delta: f32) {
        if let Some(navigation) = &mut self.scene.navigation {
            navigation.look_around(yaw_delta, pitch_delta);
        }
    }
    #[wasm_bindgen(js_name = setPointSize)]
    pub fn set_point_size(&mut self, pixels: f32) -> Result<(), JsValue> {
        self.scene.set_point_size(pixels).map_err(js_error)
    }
    #[wasm_bindgen(js_name = cameraTime)]
    pub fn camera_time(&self) -> f64 {
        self.scene
            .navigation
            .as_ref()
            .map_or(0., |n| n.timestamp_seconds())
    }
    #[wasm_bindgen(js_name = cameraIndex)]
    pub fn camera_index(&self) -> u32 {
        self.scene
            .navigation
            .as_ref()
            .map_or(0, |n| n.index() as u32)
    }
    #[wasm_bindgen(js_name = cameraCount)]
    pub fn camera_count(&self) -> u32 {
        self.scene.navigation.as_ref().map_or(0, |n| n.len() as u32)
    }
}
