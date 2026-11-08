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
            self.scene
                .resize(&self.device, width, height)
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
                return Ok(());
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
                return Err(js_error("WebGPU surface validation failed"));
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
        let radius = self.scene.control.radius();
        let half_height = if self.scene.orthographic {
            self.scene.ortho_bound
        } else {
            radius * (self.scene.camera.fovy_radians() * 0.5).tan()
        };
        self.scene
            .control
            .pointer_move([x, y], [width, height * radius / half_height]);
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self) {
        self.scene.control.pointer_up(OrbitalPointerButton::Primary);
        self.scene.control.pointer_up(OrbitalPointerButton::Middle);
    }

    pub fn scroll(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        if self.scene.orthographic {
            self.scene.ortho_bound = (self.scene.ortho_bound
                * (delta * 0.002).clamp(-80.0, 80.0).exp())
            .clamp(2.0, 30.0);
        } else {
            self.scene.control.scroll(delta);
        }
    }

    #[wasm_bindgen(js_name = resetTerrain)]
    pub fn reset_terrain(&mut self, heights: &[f32], seeds: &[u32]) -> Result<(), JsValue> {
        if heights.len() != (crate::scene::GRID * crate::scene::GRID) as usize
            || seeds.len() != crate::scene::DROPLETS as usize
            || heights
                .iter()
                .any(|h| !h.is_finite() || *h < 0.0 || *h > 1000.0)
        {
            return Err(js_error("Invalid height field or droplet seeds"));
        }
        self.scene.reset(&self.device, heights, seeds);
        Ok(())
    }

    #[wasm_bindgen(js_name = setErosion)]
    pub fn set_erosion(&mut self, values: &[f32]) -> Result<(), JsValue> {
        if values.len() != 10
            || values.iter().any(|v| !v.is_finite())
            || !(0.0..=1.0).contains(&values[0])
            || !(0.1..=20.0).contains(&values[1])
            || !(0.0..=1.0).contains(&values[2])
            || !(0.0..=1.0).contains(&values[3])
            || !(0.001..=0.2).contains(&values[4])
            || !(0.1..=20.0).contains(&values[5])
            || !(0.1..=5.0).contains(&values[6])
            || !(10.0..=500.0).contains(&values[7])
            || !(0.0..=0.25).contains(&values[8])
            || !(0.0..=1.0).contains(&values[9])
        {
            return Err(js_error("Invalid erosion settings"));
        }
        self.scene.set_erosion(&self.device, values);
        Ok(())
    }

    #[wasm_bindgen(js_name = setView)]
    pub fn set_view(&mut self, values: &[f32]) -> Result<(), JsValue> {
        if values.len() != 24 || values.iter().any(|v| !v.is_finite()) {
            return Err(js_error("Invalid view settings"));
        }
        self.scene.light_yaw = values[0];
        self.scene.light_pitch = values[1].clamp(5.0, 85.0);
        self.scene.contrast = values[2].clamp(0.1, 3.0);
        self.scene.line_count = (values[3] as u32).clamp(16, crate::scene::MAX_LINES);
        self.scene.line_width = values[4].clamp(0.5, 4.0);
        self.scene.shadows = values[5] > 0.5;
        self.scene.show_heightmap = values[6] > 0.5;
        self.scene.debug_mode = values[7].clamp(0.0, 4.0);
        self.scene.debug_gain = values[8].clamp(0.1, 10.0);
        self.scene.show_initial = values[9] > 0.5;
        self.scene.terrain_height = values[10].clamp(0.5, 5.0);
        self.scene.steps_per_frame = (values[11] as u32).clamp(1, 16);
        self.scene.step_limit = (values[12] as u32).min(50000);
        self.scene.spot_angle = values[13].clamp(0.15, 1.4);
        self.scene.spot_falloff = values[14].clamp(0.1, 8.0);
        self.scene.contour_spacing = values[15].clamp(0.02, 0.5);
        self.scene.contour_strength = values[16].clamp(0.0, 0.4);
        self.scene.layer_spacing = values[17].clamp(0.0, 10.0);
        self.scene.line_color = [
            values[18].clamp(0.0, 1.0),
            values[19].clamp(0.0, 1.0),
            values[20].clamp(0.0, 1.0),
            1.0,
        ];
        self.scene.background_color = wgpu::Color {
            r: f64::from(values[21].clamp(0.0, 1.0)),
            g: f64::from(values[22].clamp(0.0, 1.0)),
            b: f64::from(values[23].clamp(0.0, 1.0)),
            a: 1.0,
        };
        Ok(())
    }

    #[wasm_bindgen(js_name = setPaused)]
    pub fn set_paused(&mut self, paused: bool) {
        self.scene.paused = paused;
    }

    pub fn advance(&mut self, steps: u32) {
        self.scene.advance(steps);
    }
    pub fn steps(&self) -> u32 {
        self.scene.steps
    }

    #[wasm_bindgen(js_name = setCamera)]
    pub fn set_camera(&mut self, yaw: f32, pitch: f32, radius: f32) {
        if !yaw.is_finite() || !pitch.is_finite() || !radius.is_finite() {
            return;
        }
        self.scene.control.snap_yaw(yaw.to_radians());
        self.scene
            .control
            .snap_pitch(pitch.clamp(5.0, 89.0).to_radians());
        self.scene.control.snap_radius(radius.clamp(6.0, 40.0));
    }

    #[wasm_bindgen(js_name = setProjection)]
    pub fn set_projection(
        &mut self,
        orthographic: bool,
        fov: f32,
        bound: f32,
    ) -> Result<(), JsValue> {
        if !fov.is_finite() || !bound.is_finite() {
            return Err(js_error("Invalid camera projection settings"));
        }
        self.scene
            .camera
            .set_fovy_radians(fov.clamp(15.0, 90.0).to_radians())
            .map_err(js_error)?;
        self.scene.orthographic = orthographic;
        self.scene.ortho_bound = bound.clamp(2.0, 30.0);
        Ok(())
    }

    #[wasm_bindgen(js_name = cameraState)]
    pub fn camera_state(&self) -> Vec<f32> {
        let control = &self.scene.control;
        let center = control.center();
        vec![
            control.yaw().to_degrees(),
            control.pitch().to_degrees(),
            control.radius(),
            center[0],
            center[1],
            center[2],
            self.scene.ortho_bound,
        ]
    }

    #[wasm_bindgen(js_name = resetCamera)]
    pub fn reset_camera(&mut self) -> Result<(), JsValue> {
        self.scene.control = belfast::OrbitalControl::new(belfast::OrbitalControlOptions {
            center: [0.0, 0.8, 0.0],
            radius: 30.0,
            min_radius: 6.0,
            max_radius: 40.0,
            rotate_sensitivity: 0.006,
            ..Default::default()
        })
        .map_err(js_error)?;
        self.set_camera(45.64277, 28.363918, 30.0);
        self.set_projection(self.scene.orthographic, 42.0, 9.0)?;
        Ok(())
    }

    #[wasm_bindgen(js_name = readLines)]
    pub fn read_lines(&self) -> js_sys::Promise {
        let bytes = u64::from(self.scene.line_count * crate::scene::SAMPLES * 2) * 48;
        read_buffer(&self.device, self.scene.segments.gpu(), bytes)
    }

    #[wasm_bindgen(js_name = readHeights)]
    pub fn read_heights(&self) -> js_sys::Promise {
        read_buffer(
            &self.device,
            self.scene.terrain.gpu(),
            self.scene.terrain.size(),
        )
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

fn read_buffer(device: &Device, source: &wgpu::Buffer, bytes: u64) -> js_sys::Promise {
    let staging = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: Some("MountainExportReadback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, 0, &staging, 0, bytes);
    device.queue().submit([encoder.finish()]);
    let mapped = staging.clone();
    js_sys::Promise::new(&mut move |resolve, reject| {
        let buffer = mapped.clone();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| match result {
                Ok(()) => {
                    let range = match buffer.slice(..).get_mapped_range() {
                        Ok(range) => range,
                        Err(error) => {
                            let _ = reject.call1(&JsValue::UNDEFINED, &js_error(error));
                            buffer.unmap();
                            return;
                        }
                    };
                    let floats: &[f32] = bytemuck::cast_slice(&range);
                    let data = js_sys::Float32Array::from(floats);
                    drop(range);
                    buffer.unmap();
                    let _ = resolve.call1(&JsValue::UNDEFINED, &data);
                }
                Err(error) => {
                    let _ = reject.call1(&JsValue::UNDEFINED, &js_error(error));
                }
            });
    })
}
