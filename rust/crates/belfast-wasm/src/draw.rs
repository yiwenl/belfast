use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice, WasmMesh};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DrawOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

fn validate_shader_code(shader_code: &str) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(shader_code)
        .map_err(|error| format!("WGSL parse failed: {error}"))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map(|_| ())
        .map_err(|error| format!("WGSL validation failed: {error}"))
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
        validate_shader_code(shader_code).map_err(to_js_error)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const COLORED_TRIANGLE_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn vs_main(
    @location(0) position: vec2f,
    @location(1) color: vec3f,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4f(position, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return vec4f(input.color, 1.0);
}
"#;

    #[test]
    fn accepts_valid_colored_triangle_shader() {
        assert_eq!(validate_shader_code(COLORED_TRIANGLE_SHADER), Ok(()));
    }

    #[test]
    fn rejects_malformed_wgsl() {
        let error = validate_shader_code("@vertex fn vs_main(").unwrap_err();

        assert!(error.starts_with("WGSL parse failed:"), "{error}");
    }

    #[test]
    fn rejects_semantically_invalid_wgsl() {
        let error = validate_shader_code("@vertex fn vs_main() {}").unwrap_err();

        assert!(error.starts_with("WGSL validation failed:"), "{error}");
    }
}
