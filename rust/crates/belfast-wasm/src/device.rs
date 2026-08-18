use wasm_bindgen::prelude::*;

use crate::to_js_error;
#[cfg(target_arch = "wasm32")]
use crate::{WasmDraw, WasmMesh};

#[cfg(target_arch = "wasm32")]
struct CanvasTarget {
    canvas: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[wasm_bindgen(js_name = Device)]
pub struct WasmDevice {
    pub(crate) inner: belfast::Device,
    #[cfg(target_arch = "wasm32")]
    canvas_target: Option<CanvasTarget>,
}

#[wasm_bindgen(js_class = Device)]
impl WasmDevice {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: web_sys::HtmlCanvasElement) -> Result<WasmDevice, JsValue> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
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
            .ok_or_else(|| to_js_error("WebGPU adapter unavailable"))?;
        let (gpu, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BelfastDevice"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                },
                None,
            )
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
        let window = web_sys::window().ok_or_else(|| to_js_error("browser window unavailable"))?;
        let size = surface_size(
            canvas.client_width().max(0) as u32,
            canvas.client_height().max(0) as u32,
            window.device_pixel_ratio(),
        );
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
            surface.configure(&gpu, &config);
        }

        Ok(Self {
            inner: belfast::Device::from_wgpu(gpu, queue, format),
            canvas_target: Some(CanvasTarget {
                canvas,
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
        Ok(Self {
            inner,
            #[cfg(target_arch = "wasm32")]
            canvas_target: None,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn resize(&mut self) -> Result<bool, JsValue> {
        let Some(target) = self.canvas_target.as_mut() else {
            return Ok(false);
        };
        let window = web_sys::window().ok_or_else(|| to_js_error("browser window unavailable"))?;
        let Some((width, height)) = surface_size(
            target.canvas.client_width().max(0) as u32,
            target.canvas.client_height().max(0) as u32,
            window.device_pixel_ratio(),
        ) else {
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
    pub fn render(&self, draw: &WasmDraw, mesh: &WasmMesh) -> Result<(), JsValue> {
        let target = self
            .canvas_target
            .as_ref()
            .ok_or_else(|| to_js_error("cannot render with a headless device"))?;
        if target.config.width == 0 || target.config.height == 0 {
            return Ok(());
        }

        let frame = match target.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                target.surface.configure(self.inner.gpu(), &target.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(error @ wgpu::SurfaceError::OutOfMemory) => return Err(to_js_error(error)),
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
            });
            draw.inner.draw(&mut pass, mesh.inner(), 1);
        }
        self.inner.queue().submit([encoder.finish()]);
        frame.present();

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
) -> Option<(u32, u32)> {
    if client_width == 0 || client_height == 0 {
        return None;
    }

    let device_pixel_ratio = if device_pixel_ratio.is_finite() && device_pixel_ratio > 0.0 {
        device_pixel_ratio
    } else {
        1.0
    };
    let width = (f64::from(client_width) * device_pixel_ratio)
        .round()
        .max(1.0) as u32;
    let height = (f64::from(client_height) * device_pixel_ratio)
        .round()
        .max(1.0) as u32;

    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_css_size_by_device_pixel_ratio() {
        assert_eq!(surface_size(320, 180, 2.0), Some((640, 360)));
    }

    #[test]
    fn rounds_fractional_device_pixel_dimensions() {
        assert_eq!(surface_size(321, 181, 1.5), Some((482, 272)));
    }

    #[test]
    fn skips_zero_sized_canvas() {
        assert_eq!(surface_size(0, 180, 2.0), None);
        assert_eq!(surface_size(320, 0, 2.0), None);
    }

    #[test]
    fn normalizes_invalid_device_pixel_ratio() {
        assert_eq!(surface_size(320, 180, f64::NAN), Some((320, 180)));
        assert_eq!(surface_size(320, 180, 0.0), Some((320, 180)));
    }
}
