#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::{to_js_error, WasmDevice};

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ShadowMapOptionsInput {
    #[serde(default)]
    size: Option<u32>,
    #[serde(default)]
    label: Option<String>,
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct ShadowMapState {
    shadow_map: belfast::ShadowMap,
    device: belfast::Device,
}

#[cfg(target_arch = "wasm32")]
impl ShadowMapState {
    pub(crate) fn shadow_map(&self) -> &belfast::ShadowMap {
        &self.shadow_map
    }

    pub(crate) fn device(&self) -> &belfast::Device {
        &self.device
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = ShadowMap)]
pub struct WasmShadowMap {
    pub(crate) state: Rc<ShadowMapState>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = ShadowMap)]
impl WasmShadowMap {
    #[wasm_bindgen(js_name = create, unchecked_return_type = "ShadowMap")]
    pub fn create(device: &WasmDevice, options: Option<JsValue>) -> Result<JsValue, JsValue> {
        let options: ShadowMapOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let shadow_map = belfast::ShadowMap::create(
            &device.inner,
            belfast::ShadowMapOptions {
                size: options.size.unwrap_or(1024),
                label: options.label.unwrap_or_else(|| "ShadowMap".into()),
                ..Default::default()
            },
        )
        .map_err(to_js_error)?;

        let wrapper = JsValue::from(Self {
            state: Rc::new(ShadowMapState {
                shadow_map,
                device: device.inner.clone(),
            }),
        });
        crate::frame::register_shadow_map_wrapper(&wrapper)?;
        Ok(wrapper)
    }

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> u32 {
        self.state.shadow_map.size()
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmShadowMap {
        Self {
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShadowMapOptionsInput;

    #[test]
    fn shadow_map_options_default_to_empty() {
        let options = ShadowMapOptionsInput::default();
        assert_eq!(options.size, None);
        assert_eq!(options.label, None);
    }
}
