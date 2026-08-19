use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;

use crate::to_js_error;
#[cfg(target_arch = "wasm32")]
use crate::WasmDraw;

#[cfg(target_arch = "wasm32")]
struct CanvasTarget {
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
}

#[derive(Clone, Default)]
struct PendingGpuErrors {
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
    fn take(&self) -> Option<String> {
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

#[wasm_bindgen(js_name = Device)]
pub struct WasmDevice {
    pub(crate) inner: belfast::Device,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pending_gpu_errors: PendingGpuErrors,
    #[cfg(target_arch = "wasm32")]
    canvas_target: Option<CanvasTarget>,
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
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
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
            canvas_target: Some(CanvasTarget {
                canvas,
                instance,
                surface,
                config,
            }),
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
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn resize(&mut self) -> Result<bool, JsValue> {
        let max_texture_dimension_2d = self.inner.gpu().limits().max_texture_dimension_2d;
        let Some(target) = self.canvas_target.as_mut() else {
            return Ok(false);
        };
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
    pub fn render(&mut self, draw: &WasmDraw) -> Result<(), JsValue> {
        if let Some(error) = self.pending_gpu_errors.take() {
            return Err(to_js_error(error));
        }
        let mesh = draw.state.mesh();
        draw.state
            .draw()
            .validate_for_render(&self.inner, &mesh)
            .map_err(to_js_error)?;
        let target = self
            .canvas_target
            .as_mut()
            .ok_or_else(|| to_js_error("cannot render with a headless device"))?;
        if target.config.width == 0 || target.config.height == 0 {
            return Ok(());
        }

        let (frame, reconfigure_after_present) = match target.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                target.surface.configure(self.inner.gpu(), &target.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Some(error) = self.pending_gpu_errors.take() {
                    return Err(to_js_error(error));
                }
                target
                    .recreate_surface(self.inner.gpu())
                    .map_err(to_js_error)?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(to_js_error(
                    self.pending_gpu_errors
                        .take()
                        .unwrap_or_else(|| "canvas surface validation failed".into()),
                ));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.inner
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("BelfastRenderEncoder"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("BelfastRenderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.025,
                            b: 0.04,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            draw.state.draw().draw(&mut pass, &mesh, 1);
        }
        self.inner.queue().submit([encoder.finish()]);
        frame.present();
        if reconfigure_after_present {
            target.surface.configure(self.inner.gpu(), &target.config);
        }

        if let Some(error) = self.pending_gpu_errors.take() {
            return Err(to_js_error(error));
        }

        Ok(())
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
}
