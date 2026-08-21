#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::{to_js_error, WasmDevice};

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderTargetOptionsInput {
    width: u32,
    height: u32,
    #[serde(default)]
    label: Option<String>,
}

fn validate_render_target_dimensions(
    width: u32,
    height: u32,
    max_texture_dimension_2d: u32,
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("render target dimensions must be greater than zero".into());
    }
    if width > max_texture_dimension_2d || height > max_texture_dimension_2d {
        return Err(format!(
            "render target dimensions {width}x{height} exceed limit {max_texture_dimension_2d}"
        ));
    }
    Ok(())
}

fn render_target_size_changed(
    current_width: u32,
    current_height: u32,
    width: u32,
    height: u32,
) -> bool {
    current_width != width || current_height != height
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = RenderTarget)]
pub struct WasmRenderTarget {
    pub(crate) target: Rc<RefCell<belfast::RenderTarget>>,
    pub(crate) device: belfast::Device,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = RenderTarget)]
impl WasmRenderTarget {
    #[wasm_bindgen(js_name = create, unchecked_return_type = "RenderTarget")]
    pub fn create(device: &WasmDevice, options: JsValue) -> Result<JsValue, JsValue> {
        let options: RenderTargetOptionsInput =
            serde_wasm_bindgen::from_value(options).map_err(to_js_error)?;
        validate_render_target_dimensions(
            options.width,
            options.height,
            device.inner.gpu().limits().max_texture_dimension_2d,
        )
        .map_err(to_js_error)?;
        let target = belfast::RenderTarget::create(
            &device.inner,
            belfast::RenderTargetOptions {
                width: options.width,
                height: options.height,
                label: options.label.unwrap_or_else(|| "RenderTarget".into()),
                format: Some(device.inner.format()),
                sample_count: 1,
                ..Default::default()
            },
        );

        let wrapper = JsValue::from(Self {
            target: Rc::new(RefCell::new(target)),
            device: device.inner.clone(),
        });
        crate::frame::register_render_target_wrapper(&wrapper)?;
        Ok(wrapper)
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.target.borrow().width()
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.target.borrow().height()
    }

    #[wasm_bindgen(js_name = resize)]
    pub fn resize(&self, width: u32, height: u32) -> Result<bool, JsValue> {
        validate_render_target_dimensions(
            width,
            height,
            self.device.gpu().limits().max_texture_dimension_2d,
        )
        .map_err(to_js_error)?;

        let changed = {
            let target = self.target.borrow();
            render_target_size_changed(target.width(), target.height(), width, height)
        };
        if changed {
            self.target.borrow_mut().resize(width, height);
        }
        Ok(changed)
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmRenderTarget {
        Self {
            target: self.target.clone(),
            device: self.device.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_target_dimensions_must_be_positive() {
        assert_eq!(
            validate_render_target_dimensions(0, 180, 4096),
            Err("render target dimensions must be greater than zero".into())
        );
        assert_eq!(
            validate_render_target_dimensions(320, 0, 4096),
            Err("render target dimensions must be greater than zero".into())
        );
    }

    #[test]
    fn render_target_dimensions_must_fit_the_device_limit() {
        assert!(validate_render_target_dimensions(4096, 4096, 4096).is_ok());
        assert_eq!(
            validate_render_target_dimensions(4097, 2048, 4096),
            Err("render target dimensions 4097x2048 exceed limit 4096".into())
        );
    }

    #[test]
    fn resize_reports_only_dimension_changes() {
        assert!(!render_target_size_changed(320, 180, 320, 180));
        assert!(render_target_size_changed(320, 180, 640, 360));
    }
}
