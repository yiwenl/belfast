use std::sync::{Arc, Mutex};

use belfast::{Device, OrbitalPointerButton};
use wasm_bindgen::prelude::*;

use crate::scene::Scene;
use crate::sorting::{SortCamera, SplatSorter};

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
                required_limits: wgpu::Limits {
                    max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d.min(8192),
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
            self.scene.viewport = [width, height];
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

    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(&mut self, x: f32, y: f32, button: u16, pan: bool) {
        let button = match button {
            0 => OrbitalPointerButton::Primary,
            1 => OrbitalPointerButton::Middle,
            _ => return,
        };
        self.scene.control.pointer_down([x, y], button, pan);
    }

    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.scene.control.pointer_move([x, y], [width, height]);
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self) {
        self.scene.control.pointer_up(OrbitalPointerButton::Primary);
        self.scene.control.pointer_up(OrbitalPointerButton::Middle);
    }

    pub fn scroll(&mut self, delta: f32) {
        self.scene.control.scroll(delta);
    }

    #[wasm_bindgen(js_name = loadPly)]
    pub fn load_ply(&mut self, bytes: &[u8]) -> Result<u32, JsValue> {
        self.scene.load_ply(&self.device, bytes).map_err(js_error)
    }

    #[wasm_bindgen(js_name = loadGlb)]
    pub fn load_glb(&mut self, bytes: &[u8]) -> Result<u32, JsValue> {
        self.scene.load_glb(&self.device, bytes).map_err(js_error)
    }

    #[wasm_bindgen(js_name = loadPetals)]
    pub fn load_petals(&mut self, glb: &[u8], emitters_json: &[u8]) -> Result<u32, JsValue> {
        self.scene
            .load_petals(&self.device, glb, emitters_json)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setPetalCount)]
    pub fn set_petal_count(&mut self, count: u32) -> Result<u32, JsValue> {
        self.scene
            .set_petal_count(&self.device, count)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setPetalScale)]
    pub fn set_petal_scale(&self, scale: f32) -> Result<f32, JsValue> {
        self.scene
            .set_petal_scale(&self.device, scale)
            .map_err(js_error)
    }

    /// Normalized canvas coordinates keep the DOM preview aligned at any DPR.
    #[wasm_bindgen(js_name = setShadowDebugRect)]
    pub fn set_shadow_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if [x, y, width, height].iter().any(|v| !v.is_finite()) {
            return;
        }
        let x = x.clamp(0., 1.);
        let y = y.clamp(0., 1.);
        self.scene.shadow_debug_rect = [x, y, width.clamp(0., 1. - x), height.clamp(0., 1. - y)];
    }

    #[wasm_bindgen(js_name = takeSortPositions)]
    pub fn take_sort_positions(&mut self) -> Vec<f32> {
        self.scene.take_sort_positions()
    }

    #[wasm_bindgen(js_name = requestSort)]
    pub fn request_sort(&mut self) -> Vec<f32> {
        self.scene.request_sort()
    }

    #[wasm_bindgen(js_name = applySort)]
    pub fn apply_sort(&mut self, indices: &[u32], did_sort: Option<bool>) -> Result<(), JsValue> {
        self.scene
            .apply_sort(&self.device, indices, did_sort.unwrap_or(true))
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = invalidateSort)]
    pub fn invalidate_sort(&mut self) {
        self.scene.invalidate_sort();
    }

    #[wasm_bindgen(js_name = discardSort)]
    pub fn discard_sort(&mut self) {
        self.scene.discard_sort();
    }

    #[wasm_bindgen(js_name = sortCount)]
    pub fn sort_count(&self) -> u32 {
        self.scene.sort_count
    }

    #[wasm_bindgen(js_name = visibleCount)]
    pub fn visible_count(&self) -> u32 {
        self.scene.visible_count()
    }

    #[wasm_bindgen(js_name = sourceCount)]
    pub fn source_count(&self) -> u32 {
        self.scene.source_count
    }

    #[wasm_bindgen(js_name = setSkyPreset)]
    pub fn set_sky_preset(&mut self, preset: &str) -> Result<(), JsValue> {
        self.scene.sky.set_preset(preset).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setSkyImage)]
    pub fn set_sky_image(&mut self, rgba: &[u8], width: u32, height: u32) -> Result<(), JsValue> {
        self.scene
            .sky
            .set_image(&self.device, rgba, width, height)
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

/// CPU-only Rust sorter; constructed in a Web Worker, with no canvas or GPU device.
#[wasm_bindgen]
pub struct GaussianSorter {
    inner: SplatSorter,
}

#[wasm_bindgen]
impl GaussianSorter {
    #[wasm_bindgen(constructor)]
    pub fn new(positions: &[f32]) -> Result<GaussianSorter, JsValue> {
        if positions.is_empty()
            || !positions.len().is_multiple_of(4)
            || !positions.iter().all(|n| n.is_finite())
            || positions.as_chunks::<4>().0.iter().any(|p| p[3] < 0.)
        {
            return Err(js_error("Expected finite Gaussian XYZ/radius records"));
        }
        Ok(Self {
            inner: SplatSorter::new(
                positions
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| [p[0], p[1], p[2], p[3]])
                    .collect(),
            ),
        })
    }

    pub fn sort(&mut self, camera: &[f32]) -> Result<Vec<u32>, JsValue> {
        Ok(self
            .inner
            .sort(&SortCamera::decode(camera).map_err(js_error)?)
            .to_vec())
    }

    #[wasm_bindgen(js_name = setFrozen)]
    pub fn set_frozen(&mut self, frozen: bool) {
        self.inner.set_frozen(frozen);
    }

    #[wasm_bindgen(js_name = didSort)]
    pub fn did_sort(&self) -> bool {
        self.inner.did_sort()
    }
}
