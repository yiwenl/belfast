use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice, WasmMesh};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DrawOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

#[wasm_bindgen(js_name = Draw)]
pub struct WasmDraw {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) inner: belfast::Draw,
}

#[wasm_bindgen(js_class = Draw)]
impl WasmDraw {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        shader_code: &str,
        mesh: &WasmMesh,
        options: JsValue,
    ) -> Result<WasmDraw, JsValue> {
        let options: DrawOptionsInput =
            serde_wasm_bindgen::from_value(options).map_err(to_js_error)?;
        let inner = belfast::Draw::new(
            &device.inner,
            shader_code,
            mesh.inner(),
            belfast::DrawOptions::new(
                options.label.as_deref().unwrap_or("Draw"),
                device.inner.format(),
            ),
        );

        Ok(Self { inner })
    }
}
