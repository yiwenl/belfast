use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(any(target_arch = "wasm32", test))]
use std::{cell::Cell, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::to_js_error;
#[cfg(target_arch = "wasm32")]
use crate::{WasmDraw, WasmFrame};

#[cfg(any(target_arch = "wasm32", test))]
const ACTIVE_FRAME_SURFACE_ERROR: &str = "canvas surface is owned by an active frame";

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Default)]
pub(crate) struct SurfaceLease {
    active: Rc<Cell<bool>>,
}

#[cfg(any(target_arch = "wasm32", test))]
impl SurfaceLease {
    pub(crate) fn ensure_available(&self) -> Result<(), String> {
        if self.active.get() {
            Err(ACTIVE_FRAME_SURFACE_ERROR.into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn acquire(&self) -> Result<SurfaceLeaseGuard, String> {
        self.ensure_available()?;
        self.active.set(true);
        Ok(SurfaceLeaseGuard {
            lease: self.clone(),
            held: true,
        })
    }
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) struct SurfaceLeaseGuard {
    lease: SurfaceLease,
    held: bool,
}

#[cfg(any(target_arch = "wasm32", test))]
impl SurfaceLeaseGuard {
    pub(crate) fn release(&mut self) {
        if self.held {
            self.lease.active.set(false);
            self.held = false;
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl Drop for SurfaceLeaseGuard {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct CanvasTarget {
    canvas: web_sys::HtmlCanvasElement,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[cfg(target_arch = "wasm32")]
impl CanvasTarget {
    fn recreate_surface(&mut self, device: &wgpu::Device) -> Result<(), String> {
        let surface = self
            .instance
            .create_surface(wgpu::SurfaceTarget::Canvas(self.canvas.clone()))
            .map_err(|error| format!("failed to recreate canvas surface: {error}"))?;
        surface.configure(device, &self.config);
        self.surface = surface;
        Ok(())
    }

    pub(crate) fn configure(&self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
    }
}

#[derive(Clone, Default)]
pub(crate) struct PendingGpuErrors {
    message: Arc<Mutex<Option<String>>>,
}

impl PendingGpuErrors {
    fn record(&self, error: wgpu::Error) {
        let message = match error {
            wgpu::Error::OutOfMemory { .. } => "WebGPU device ran out of memory".to_owned(),
            wgpu::Error::Validation { description, .. } => {
                format!("WebGPU validation error: {description}")
            }
            wgpu::Error::Internal { description, .. } => {
                format!("WebGPU internal error: {description}")
            }
        };
        self.record_message(message);
    }

    fn record_message(&self, message: String) {
        let mut pending = self
            .message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pending.is_none() {
            *pending = Some(message);
        }
    }

    #[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
    pub(crate) fn take(&self) -> Option<String> {
        self.message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

fn install_gpu_error_handlers(device: &wgpu::Device, pending: &PendingGpuErrors) {
    let uncaptured = pending.clone();
    device.on_uncaptured_error(Arc::new(move |error| uncaptured.record(error)));

    let lost = pending.clone();
    device.set_device_lost_callback(move |reason, message| {
        lost.record_message(format!("WebGPU device lost ({reason:?}): {message}"));
    });
}

#[cfg(any(target_arch = "wasm32", test))]
fn begin_after_draw_validation<T, E>(
    validation: Result<(), E>,
    begin: impl FnOnce() -> Result<Option<T>, E>,
) -> Result<Option<T>, E> {
    validation?;
    begin()
}

#[cfg(target_arch = "wasm32")]
fn validate_compatibility_draw(device: &belfast::Device, draw: &WasmDraw) -> Result<(), JsValue> {
    let mesh = draw.state.mesh();
    draw.state
        .draw()
        .validate_for_render(device, &mesh)
        .map_err(to_js_error)
}

#[wasm_bindgen(js_name = Device)]
pub struct WasmDevice {
    pub(crate) inner: belfast::Device,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pending_gpu_errors: PendingGpuErrors,
    #[cfg(target_arch = "wasm32")]
    canvas_target: Option<Rc<RefCell<CanvasTarget>>>,
    #[cfg(target_arch = "wasm32")]
    surface_lease: SurfaceLease,
}

#[wasm_bindgen(js_class = Device)]
impl WasmDevice {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: web_sys::HtmlCanvasElement) -> Result<WasmDevice, JsValue> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(to_js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| to_js_error("WebGPU adapter unavailable"))?;
        let (gpu, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("BelfastDevice"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(to_js_error)?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| to_js_error("canvas surface has no supported formats"))?;
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| to_js_error("canvas surface has no supported alpha modes"))?;
        let inner = belfast::Device::from_wgpu(gpu, queue, format);
        let pending_gpu_errors = PendingGpuErrors::default();
        install_gpu_error_handlers(inner.gpu(), &pending_gpu_errors);
        let max_texture_dimension_2d = inner.gpu().limits().max_texture_dimension_2d;
        let window = web_sys::window().ok_or_else(|| to_js_error("browser window unavailable"))?;
        let size = surface_size(
            canvas.client_width().max(0) as u32,
            canvas.client_height().max(0) as u32,
            window.device_pixel_ratio(),
        )
        .and_then(|size| check_surface_size_limit(size, max_texture_dimension_2d))
        .map_err(to_js_error)?;
        let (width, height) = size.unwrap_or((0, 0));
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };
        if size.is_some() {
            canvas.set_width(width);
            canvas.set_height(height);
            surface.configure(inner.gpu(), &config);
        }

        Ok(Self {
            inner,
            pending_gpu_errors,
            canvas_target: Some(Rc::new(RefCell::new(CanvasTarget {
                canvas,
                instance,
                surface,
                config,
            }))),
            surface_lease: SurfaceLease::default(),
        })
    }

    #[wasm_bindgen(js_name = createHeadless)]
    pub async fn create_headless() -> Result<WasmDevice, JsValue> {
        let inner = belfast::Device::create_headless()
            .await
            .map_err(to_js_error)?;
        let pending_gpu_errors = PendingGpuErrors::default();
        install_gpu_error_handlers(inner.gpu(), &pending_gpu_errors);
        Ok(Self {
            inner,
            pending_gpu_errors,
            #[cfg(target_arch = "wasm32")]
            canvas_target: None,
            #[cfg(target_arch = "wasm32")]
            surface_lease: SurfaceLease::default(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn resize(&mut self) -> Result<bool, JsValue> {
        self.surface_lease.ensure_available().map_err(to_js_error)?;
        let max_texture_dimension_2d = self.inner.gpu().limits().max_texture_dimension_2d;
        let Some(target) = self.canvas_target.as_ref() else {
            return Ok(false);
        };
        let mut target = target.borrow_mut();
        let window = web_sys::window().ok_or_else(|| to_js_error("browser window unavailable"))?;
        let Some((width, height)) = surface_size(
            target.canvas.client_width().max(0) as u32,
            target.canvas.client_height().max(0) as u32,
            window.device_pixel_ratio(),
        )
        .and_then(|size| check_surface_size_limit(size, max_texture_dimension_2d))
        .map_err(to_js_error)?
        else {
            return Ok(false);
        };

        let dimensions_changed = target.canvas.width() != width
            || target.canvas.height() != height
            || target.config.width != width
            || target.config.height != height;
        if dimensions_changed {
            target.canvas.set_width(width);
            target.canvas.set_height(height);
            target.config.width = width;
            target.config.height = height;
            target.surface.configure(self.inner.gpu(), &target.config);
        }

        Ok(true)
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = beginFrame)]
    pub fn begin_frame(&mut self) -> Result<Option<WasmFrame>, JsValue> {
        if let Some(error) = self.pending_gpu_errors.take() {
            return Err(to_js_error(error));
        }
        let surface_lease = self.surface_lease.acquire().map_err(to_js_error)?;
        let target = self
            .canvas_target
            .as_ref()
            .cloned()
            .ok_or_else(|| to_js_error("cannot render with a headless device"))?;
        {
            let target = target.borrow();
            if target.config.width == 0 || target.config.height == 0 {
                return Ok(None);
            }
        }

        let current_texture = target.borrow().surface.get_current_texture();
        let (frame, reconfigure_after_present) = match current_texture {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                target.borrow().configure(self.inner.gpu());
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Some(error) = self.pending_gpu_errors.take() {
                    return Err(to_js_error(error));
                }
                target
                    .borrow_mut()
                    .recreate_surface(self.inner.gpu())
                    .map_err(to_js_error)?;
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(to_js_error(
                    self.pending_gpu_errors
                        .take()
                        .unwrap_or_else(|| "canvas surface validation failed".into()),
                ));
            }
        };

        Ok(Some(WasmFrame::new(
            self.inner.clone(),
            self.pending_gpu_errors.clone(),
            target,
            frame,
            surface_lease,
            reconfigure_after_present,
        )))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn render(&mut self, draw: &WasmDraw) -> Result<(), JsValue> {
        if let Some(error) = self.pending_gpu_errors.take() {
            return Err(to_js_error(error));
        }
        let validation = validate_compatibility_draw(&self.inner, draw);
        let Some(mut frame) = begin_after_draw_validation(validation, || self.begin_frame())?
        else {
            return Ok(());
        };
        frame.render(draw, JsValue::UNDEFINED)?;
        frame.submit()
    }

    pub fn format(&self) -> String {
        format!("{:?}", self.inner.format())
    }
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn surface_size(
    client_width: u32,
    client_height: u32,
    device_pixel_ratio: f64,
) -> Result<Option<(u32, u32)>, String> {
    if client_width == 0 || client_height == 0 {
        return Ok(None);
    }

    let device_pixel_ratio = if device_pixel_ratio.is_finite() && device_pixel_ratio > 0.0 {
        device_pixel_ratio
    } else {
        1.0
    };
    let width = (f64::from(client_width) * device_pixel_ratio)
        .round()
        .max(1.0);
    let height = (f64::from(client_height) * device_pixel_ratio)
        .round()
        .max(1.0);
    if width > f64::from(u32::MAX) || height > f64::from(u32::MAX) {
        return Err(surface_size_limit_error(width, height, u32::MAX));
    }

    Ok(Some((width as u32, height as u32)))
}

#[cfg(any(target_arch = "wasm32", test))]
fn check_surface_size_limit(
    size: Option<(u32, u32)>,
    max_texture_dimension_2d: u32,
) -> Result<Option<(u32, u32)>, String> {
    if let Some((width, height)) = size {
        if width > max_texture_dimension_2d || height > max_texture_dimension_2d {
            return Err(surface_size_limit_error(
                f64::from(width),
                f64::from(height),
                max_texture_dimension_2d,
            ));
        }
    }

    Ok(size)
}

#[cfg(any(target_arch = "wasm32", test))]
fn surface_size_limit_error(width: f64, height: f64, limit: u32) -> String {
    format!("surface dimensions {width:.0}x{height:.0} exceed limit {limit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_css_size_by_device_pixel_ratio() {
        assert_eq!(surface_size(320, 180, 2.0), Ok(Some((640, 360))));
    }

    #[test]
    fn rounds_fractional_device_pixel_dimensions() {
        assert_eq!(surface_size(321, 181, 1.5), Ok(Some((482, 272))));
    }

    #[test]
    fn skips_zero_sized_canvas() {
        assert_eq!(surface_size(0, 180, 2.0), Ok(None));
        assert_eq!(surface_size(320, 0, 2.0), Ok(None));
    }

    #[test]
    fn normalizes_invalid_device_pixel_ratio() {
        assert_eq!(surface_size(320, 180, f64::NAN), Ok(Some((320, 180))));
        assert_eq!(surface_size(320, 180, 0.0), Ok(Some((320, 180))));
    }

    #[test]
    fn rejects_finite_dpr_that_overflows_u32() {
        assert_eq!(
            surface_size(2, 1, f64::from(u32::MAX)),
            Err("surface dimensions 8589934590x4294967295 exceed limit 4294967295".into())
        );
    }

    #[test]
    fn rejects_dimensions_above_max_texture_dimension() {
        assert_eq!(
            check_surface_size_limit(Some((4097, 2048)), 4096),
            Err("surface dimensions 4097x2048 exceed limit 4096".into())
        );
    }

    #[test]
    fn captures_uncaptured_gpu_errors_for_the_next_api_boundary() {
        let pending = PendingGpuErrors::default();
        pending.record(wgpu::Error::Validation {
            source: Box::new(std::io::Error::other("validation source")),
            description: "bad pipeline".into(),
        });

        assert_eq!(
            pending.take().as_deref(),
            Some("WebGPU validation error: bad pipeline")
        );
        assert_eq!(pending.take(), None);
    }

    #[test]
    fn labels_out_of_memory_as_a_fatal_gpu_error() {
        let pending = PendingGpuErrors::default();
        pending.record(wgpu::Error::OutOfMemory {
            source: Box::new(std::io::Error::other("oom source")),
        });

        assert_eq!(
            pending.take().as_deref(),
            Some("WebGPU device ran out of memory")
        );
    }

    #[test]
    fn surface_lease_rejects_overlap_until_the_active_guard_drops() {
        let lease = SurfaceLease::default();
        let guard = lease.acquire().unwrap();

        assert_eq!(
            lease.acquire().err().as_deref(),
            Some("canvas surface is owned by an active frame")
        );
        assert_eq!(
            lease.ensure_available().err().as_deref(),
            Some("canvas surface is owned by an active frame")
        );

        drop(guard);
        assert!(lease.acquire().is_ok());
    }

    #[test]
    fn surface_lease_can_be_released_before_reconfiguration() {
        let lease = SurfaceLease::default();
        let mut guard = lease.acquire().unwrap();

        guard.release();
        guard.release();

        assert!(lease.ensure_available().is_ok());
        assert!(lease.acquire().is_ok());
    }

    #[test]
    fn compatibility_render_validation_precedes_surface_acquisition() {
        let surface_was_acquired = Cell::new(false);

        let result: Result<Option<()>, String> =
            begin_after_draw_validation(Err("invalid draw".into()), || {
                surface_was_acquired.set(true);
                Ok(Some(()))
            });

        assert_eq!(result, Err("invalid draw".into()));
        assert!(!surface_was_acquired.get());
    }
}
