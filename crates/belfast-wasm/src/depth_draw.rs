use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::{
    draw::{mesh_shader_attributes, parse_and_validate_depth_shader, ShaderResourceLayout},
    to_js_error, WasmDevice, WasmMesh,
};

#[wasm_bindgen(typescript_custom_section)]
const DEPTH_DRAW_OPTION_TYPES: &str = r#"
export interface DepthDrawOptions {
  label?: string;
}
"#;

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DepthDrawOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

pub(crate) struct DepthDrawState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    draw: belfast::DepthDraw,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    mesh: RefCell<belfast::Mesh>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) resources: ShaderResourceLayout,
}

impl DepthDrawState {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn draw(&self) -> &belfast::DepthDraw {
        &self.draw
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn mesh(&self) -> std::cell::Ref<'_, belfast::Mesh> {
        self.mesh.borrow()
    }
}

#[wasm_bindgen(js_name = DepthDraw)]
pub struct WasmDepthDraw {
    pub(crate) state: Rc<DepthDrawState>,
}

#[wasm_bindgen(js_class = DepthDraw)]
impl WasmDepthDraw {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        shader_code: &str,
        mesh: &WasmMesh,
        #[wasm_bindgen(unchecked_optional_param_type = "DepthDrawOptions")] options: Option<
            JsValue,
        >,
    ) -> Result<WasmDepthDraw, JsValue> {
        let options: DepthDrawOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let mesh = mesh.clone_inner();
        belfast::Draw::validate_mesh_device(&device.inner, &mesh).map_err(to_js_error)?;
        let mesh_attributes = mesh_shader_attributes(&mesh);
        let resources = parse_and_validate_depth_shader(
            shader_code,
            &mesh_attributes,
            device.inner.gpu().limits().max_inter_stage_shader_variables,
        )
        .map_err(to_js_error)?;
        let label = options.label.unwrap_or_else(|| "DepthDraw".to_owned());
        let draw = belfast::DepthDraw::new(
            &device.inner,
            shader_code,
            &mesh,
            belfast::DepthDrawOptions::new(&label),
        );

        Ok(Self {
            state: Rc::new(DepthDrawState {
                draw,
                mesh: RefCell::new(mesh),
                resources,
            }),
        })
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmDepthDraw {
        Self {
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::draw::parse_and_validate_depth_shader;

    const DEPTH_SHADER: &str = r#"
struct LightUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> light: LightUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) local: vec3<f32>,
    @location(1) instance_pos_size: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = light.view_proj * vec4<f32>(local * instance_pos_size.w + instance_pos_size.xyz, 1.0);
    output.local = local.xy;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) {
    if (length(input.local * 2.0) > 1.0) {
        discard;
    }
}
"#;

    const COLOR_OUTPUT_SHADER: &str = r#"
@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    #[test]
    fn accepts_void_fragment_depth_shader() {
        let layout = parse_and_validate_depth_shader(
            DEPTH_SHADER,
            &[
                (0, wgpu::VertexFormat::Float32x3),
                (1, wgpu::VertexFormat::Float32x4),
            ],
            16,
        )
        .unwrap();
        assert!(matches!(
            layout,
            crate::draw::ShaderResourceLayout::Uniform {
                group: 0,
                binding: 0
            }
        ));
    }

    #[test]
    fn rejects_color_output_from_depth_shader() {
        assert_eq!(
            parse_and_validate_depth_shader(COLOR_OUTPUT_SHADER, &[], 16),
            Err("depth fragment shader must not output color".into())
        );
    }
}
