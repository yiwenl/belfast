use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

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
    benchmark_frame: Option<u32>,
    gpu_idle: Arc<AtomicBool>,
    adapter_name: String,
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
        let optional_features = adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::FLOAT32_BLENDABLE);
        let adapter_name = adapter.get_info().name;
        let (gpu, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ExperimentDevice"),
                required_features: optional_features,
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
            // Preserve the cause instead of a later invalid-command-buffer cascade.
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
        // On WebGPU, this selects an fp16 canvas with extended tone mapping
        // when supported, and keeps the existing SDR format as a fallback.
        let color = belfast::pick_surface_color(&capabilities, true);
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
            benchmark_frame: None,
            gpu_idle: Arc::new(AtomicBool::new(true)),
            adapter_name,
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
        let previous = self.last_frame_ms.unwrap_or(timestamp_ms);
        let delta_seconds = ((timestamp_ms - previous) * 0.001).clamp(0.0, 0.1) as f32;
        self.last_frame_ms = Some(timestamp_ms.max(previous));
        self.render_frame(delta_seconds, timestamp_ms)?;
        Ok(())
    }

    fn render_frame(&mut self, delta_seconds: f32, timestamp_ms: f64) -> Result<bool, JsValue> {
        self.check_gpu_error()?;
        if !self.drawable {
            return Ok(false);
        }
        let (frame, reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
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
                return Err(js_error("WebGPU surface validation failed"));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Acquire first: an unavailable surface must not advance a benchmark step.
        self.scene.update(&self.device, delta_seconds, timestamp_ms);
        self.scene.render(&self.device, &view);
        self.device.queue().present(frame);
        if reconfigure {
            self.surface.configure(self.device.gpu(), &self.config);
        }
        Ok(true)
    }

    #[wasm_bindgen(js_name = setViewCulling)]
    pub fn set_view_culling(&mut self, enabled: bool) {
        self.scene.set_view_culling(enabled);
    }

    #[wasm_bindgen(js_name = checkGpuError)]
    pub fn check_gpu_error(&self) -> Result<(), JsValue> {
        if let Some(error) = self.gpu_error.lock().unwrap().take() {
            return Err(js_error(error));
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = gpuTimingSupported)]
    pub fn gpu_timing_supported(&self) -> bool {
        self.device
            .gpu()
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    }

    #[wasm_bindgen(js_name = mergedOcclusion)]
    pub fn merged_occlusion(&self) -> bool {
        crate::fireflies::merged_depth(&self.device)
    }

    #[wasm_bindgen(js_name = benchmarkEnvironment)]
    pub fn benchmark_environment(&self) -> String {
        format!(
            "{} / {:?} / {:?}",
            self.adapter_name, self.config.format, self.config.color_space
        )
    }

    #[wasm_bindgen(js_name = beginBenchmark)]
    pub fn begin_benchmark(&mut self, seed: u32, moving: bool) {
        self.scene.begin_benchmark(&self.device, seed, moving);
        self.benchmark_frame = Some(0);
    }

    #[wasm_bindgen(js_name = resetBenchmark)]
    pub fn reset_benchmark(&mut self, seed: u32, frames: u32) -> Result<(), JsValue> {
        if self.benchmark_frame.is_none() || !(1..=10_000).contains(&frames) {
            return Err(js_error("Benchmark must be active with 1–10000 frames"));
        }
        self.scene.reset_benchmark(&self.device, seed, frames);
        self.benchmark_frame = Some(0);
        self.flush_gpu();
        Ok(())
    }

    #[wasm_bindgen(js_name = benchmarkFrame)]
    pub fn benchmark_frame(&mut self) -> Result<bool, JsValue> {
        let frame = self
            .benchmark_frame
            .ok_or_else(|| js_error("Benchmark is not active"))?;
        let rendered = self.render_frame(1. / 60., f64::from(frame) * (1000. / 60.))?;
        if rendered {
            self.benchmark_frame = Some(frame + 1);
        }
        Ok(rendered)
    }

    #[wasm_bindgen(js_name = benchmarkReady)]
    pub fn benchmark_ready(&self) -> bool {
        self.scene.benchmark_ready()
    }

    #[wasm_bindgen(js_name = benchmarkCamera)]
    pub fn benchmark_camera(&self) -> Vec<f32> {
        self.scene.camera_snapshot()
    }

    #[wasm_bindgen(js_name = finishGpuTiming)]
    pub fn finish_gpu_timing(&mut self) {
        self.scene.finish_gpu_timing(&self.device);
        self.flush_gpu();
    }

    fn flush_gpu(&mut self) {
        let idle = Arc::new(AtomicBool::new(false));
        self.gpu_idle = idle.clone();
        self.device.queue().submit([]);
        self.device
            .queue()
            .on_submitted_work_done(move || idle.store(true, Ordering::Release));
    }

    #[wasm_bindgen(js_name = gpuIdle)]
    pub fn gpu_idle(&self) -> bool {
        self.gpu_idle.load(Ordering::Acquire)
    }

    #[wasm_bindgen(js_name = takeGpuTimings)]
    pub fn take_gpu_timings(&mut self) -> Result<Vec<f64>, JsValue> {
        self.scene.take_gpu_timings().map_err(js_error)
    }

    #[wasm_bindgen(js_name = endBenchmark)]
    pub fn end_benchmark(&mut self) {
        self.scene.end_benchmark();
        self.benchmark_frame = None;
        self.last_frame_ms = None;
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

    #[wasm_bindgen(js_name = hdrEnabled)]
    pub fn hdr_enabled(&self) -> bool {
        self.device.hdr()
    }

    #[wasm_bindgen(js_name = setCameraFov)]
    pub fn set_camera_fov(&mut self, degrees: f32) -> Result<f32, JsValue> {
        self.scene.set_camera_fov(degrees).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setFireflyRates)]
    pub fn set_firefly_rates(&mut self, movement: f32, blinking: f32) -> Result<(), JsValue> {
        self.scene
            .set_firefly_rates(movement, blinking)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setFireflyCount)]
    pub fn set_firefly_count(&mut self, count: u32) -> u32 {
        self.scene.set_firefly_count(count)
    }

    #[wasm_bindgen(js_name = setFireflyFlashDuration)]
    pub fn set_firefly_flash_duration(&mut self, seconds: f32) -> Result<(), JsValue> {
        self.scene
            .set_firefly_flash_duration(seconds)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = loadPly)]
    pub fn load_ply(&mut self, bytes: &[u8]) -> Result<u32, JsValue> {
        self.scene.load_ply(&self.device, bytes).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setForestGrade)]
    pub fn set_forest_grade(
        &self,
        strength: f32,
        brightness: f32,
        colors: &[f32],
    ) -> Result<(), JsValue> {
        self.scene
            .set_forest_grade(&self.device, strength, brightness, colors)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setFireflyAppearance)]
    pub fn set_firefly_appearance(
        &mut self,
        min_peak: f32,
        max_peak: f32,
        warm_share: f32,
        colors: &[f32],
    ) -> Result<(), JsValue> {
        self.scene
            .set_firefly_appearance(&self.device, min_peak, max_peak, warm_share, colors)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setLightVolume)]
    pub fn set_light_volume(
        &mut self,
        enabled: bool,
        strength: f32,
        radius: f32,
        hz: f32,
    ) -> Result<(), JsValue> {
        self.scene
            .set_light_volume(&self.device, enabled, strength, radius, hz)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setFog)]
    pub fn set_fog(
        &mut self,
        enabled: bool,
        density: f32,
        speed: f32,
        light: f32,
    ) -> Result<(), JsValue> {
        self.scene
            .set_fog(enabled, density, speed, light)
            .map_err(js_error)
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
    pub fn apply_sort(&mut self, indices: &[u32]) -> Result<(), JsValue> {
        self.scene
            .apply_sort(&self.device, indices)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = visibleCount)]
    pub fn visible_count(&self) -> u32 {
        self.scene.visible_count()
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

/// CPU-only sorter instantiated in the worker, without a canvas or GPU device.
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
            || positions.chunks_exact(4).any(|p| p[3] < 0.)
        {
            return Err(js_error("Expected finite Gaussian XYZ/radius records"));
        }
        Ok(Self {
            inner: SplatSorter::new(
                positions
                    .chunks_exact(4)
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
}
