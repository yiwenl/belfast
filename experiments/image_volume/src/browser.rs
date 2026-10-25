use std::sync::{Arc, Mutex};

use belfast::{Device, OrbitalPointerButton};
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
        let adapter_limits = adapter.limits();
        let (gpu, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ExperimentDevice"),
                required_limits: wgpu::Limits {
                    // downlevel_defaults is 256³ and a 256 MB buffer. Dawn stages the whole
                    // 1024×1024×190 volume (760 MB) on submit, and this adapter allows ~4 GB.
                    max_texture_dimension_3d: 1024.min(adapter_limits.max_texture_dimension_3d),
                    max_buffer_size: adapter_limits.max_buffer_size,
                    ..wgpu::Limits::downlevel_defaults()
                },
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
            self.scene
                .camera
                .set_aspect(width as f32 / height as f32)
                .map_err(js_error)?;
        }
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

    #[wasm_bindgen(js_name = uploadFrame)]
    pub fn upload_frame(&mut self, index: u32, pixels: &[u8]) -> Result<(), JsValue> {
        self.scene
            .upload_frame(&self.device, index, pixels)
            .map_err(js_error)
    }

    /// Face-on keeps the plane square on screen. Off shows the tilted plane in the cube.
    #[wasm_bindgen(js_name = setFaceOn)]
    pub fn set_face_on(&mut self, face_on: bool) {
        self.scene.set_face_on(face_on);
    }

    /// Show every photograph. This also turns face-on off.
    #[wasm_bindgen(js_name = setShowSlices)]
    pub fn set_show_slices(&mut self, show: bool) {
        self.scene.set_show_slices(show);
    }

    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(&mut self, x: f32, y: f32, button: u16, _pan: bool) {
        let button = match button {
            0 => OrbitalPointerButton::Primary,
            1 => OrbitalPointerButton::Middle,
            _ => return,
        };
        // Drag rotates the plane. The camera stays in front and only zooms.
        self.scene.plane_control.pointer_down([x, y], button, false);
    }

    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.scene
            .plane_control
            .pointer_move([x, y], [width, height]);
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self) {
        self.scene
            .plane_control
            .pointer_up(OrbitalPointerButton::Primary);
        self.scene
            .plane_control
            .pointer_up(OrbitalPointerButton::Middle);
    }

    pub fn scroll(&mut self, delta: f32) {
        self.scene.control.scroll(delta);
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
