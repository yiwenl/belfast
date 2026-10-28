use std::sync::{Arc, Mutex};

use belfast::Device;
use wasm_bindgen::prelude::*;

use crate::scene::{Scene, Settings};

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// A Rust-owned scene. JS supplies browser events, generated images and frame timestamps.
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
                required_limits: wgpu::Limits::default(),
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
        let view_format = color.format.add_srgb_suffix();
        let device = Device::from_wgpu(gpu, queue, view_format, Some(color.hdr));
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: color.format,
            color_space: color.color_space,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![view_format],
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

    /// Canvas dimensions in CSS pixels, scaled by the supplied device pixel ratio.
    pub fn resize(&mut self, width: u32, height: u32, dpr: f64) -> Result<(), JsValue> {
        if width == 0 || height == 0 {
            self.drawable = false;
            self.scene.clear_pointer();
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
                .resize(&self.device, width, height, scale as f32)
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
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.device.format()),
            ..Default::default()
        });
        self.scene
            .frame(&self.device, &view, delta_seconds)
            .map_err(js_error)?;
        self.device.queue().present(frame);
        if reconfigure {
            self.surface.configure(self.device.gpu(), &self.config);
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = setSettings)]
    pub fn set_settings(&mut self, values: &[f32]) -> Result<(), JsValue> {
        if values.len() != 10 {
            return Err(js_error("Expected ten blocks settings"));
        }
        if !values[3].is_finite() || values[3].fract() != 0.0 || !(1.0..=80.0).contains(&values[3])
        {
            return Err(js_error("Pressure iterations must be an integer in 1..80"));
        }
        self.scene
            .set_settings(Settings {
                flow_strength: values[0],
                flow_radius: values[1],
                dissipation: values[2],
                pressure_iterations: values[3] as u32,
                response: values[4],
                depth_motion: values[5],
                block_size: values[6],
                shadow_strength: values[7],
                mouse_strength: values[8],
                mouse_radius: values[9],
            })
            .map_err(js_error)
    }
    #[wasm_bindgen(js_name = setPaused)]
    pub fn set_paused(&mut self, paused: bool) {
        self.scene.set_paused(paused);
    }
    #[wasm_bindgen(js_name = setColorMap)]
    pub fn set_color_map(&mut self, bitmap: &web_sys::ImageBitmap) -> Result<(), JsValue> {
        if self.scene.is_color_transitioning() {
            return Err(js_error("Wait for the current color transition to finish"));
        }
        let width = bitmap.width();
        let height = bitmap.height();
        let texture = belfast::Texture::create_2d(
            &self.device,
            width,
            height,
            belfast::TextureOptions {
                label: "GeneratedBlocksPalette".into(),
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ..Default::default()
            },
        )
        .map_err(js_error)?;
        self.device.queue().copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(bitmap.clone()),
                origin: wgpu::Origin2d::ZERO,
                // The particle shader already flips the image's Y coordinate.
                flip_y: false,
            },
            wgpu::CopyExternalImageDestInfo {
                texture: texture.gpu(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.scene
            .set_color_map(&self.device, texture)
            .map_err(js_error)
    }
    #[wasm_bindgen(js_name = isColorTransitioning)]
    pub fn is_color_transitioning(&self) -> bool {
        self.scene.is_color_transitioning()
    }
    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.scene.pointer_move(x, y, width, height);
    }
    #[wasm_bindgen(js_name = clearPointer)]
    pub fn clear_pointer(&mut self) {
        self.scene.clear_pointer();
    }
    #[wasm_bindgen(js_name = setDebugMaps)]
    pub fn set_debug_maps(&mut self, visible: bool) {
        self.scene.show_debug_maps = visible;
    }
    pub fn reset(&mut self) {
        self.scene.reset(&self.device);
    }
}
