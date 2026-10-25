use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{bindings, to_js_error, WasmDevice};

#[wasm_bindgen(typescript_custom_section)]
const BALL_HELPER_OPTION_TYPES: &str = r#"
export interface BallHelperOptions {
  position?: [number, number, number];
  radius?: number;
  color?: [number, number, number, number];
  label?: string;
  depth?: boolean;
}
"#;

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BallHelperOptionsInput {
    #[serde(default)]
    position: Option<[f32; 3]>,
    #[serde(default)]
    radius: Option<f32>,
    #[serde(default)]
    color: Option<[f32; 4]>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    depth: bool,
}

pub(crate) struct BallHelperState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    helper: belfast::BallHelper,
}

impl BallHelperState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn helper(&self) -> &belfast::BallHelper {
        &self.helper
    }
}

#[wasm_bindgen(js_name = BallHelper)]
pub struct WasmBallHelper {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) state: Rc<BallHelperState>,
}

#[wasm_bindgen(js_class = BallHelper)]
impl WasmBallHelper {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        #[wasm_bindgen(unchecked_optional_param_type = "BallHelperOptions")] options: Option<
            JsValue,
        >,
    ) -> Result<WasmBallHelper, JsValue> {
        let options: BallHelperOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let label = options.label.unwrap_or_else(|| "BallHelper".to_owned());
        let position = options.position.unwrap_or([0.0, 0.0, 0.0]);
        let radius = options.radius.unwrap_or(1.0);
        let color = options.color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
        let bind_group_layout =
            device
                .inner
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{label}BindGroupLayout")),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let pipeline_layout =
            device
                .inner
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&format!("{label}PipelineLayout")),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });
        let helper = belfast::BallHelper::new(
            &device.inner,
            belfast::BallHelperOptions {
                label: &label,
                position,
                radius,
                color,
                depth_stencil: options.depth.then(bindings::default_depth_state),
                ..belfast::BallHelperOptions::new(device.inner.format(), &pipeline_layout)
            },
        )
        .map_err(to_js_error)?;

        Ok(Self {
            state: Rc::new(BallHelperState { helper }),
        })
    }

    #[wasm_bindgen]
    pub fn set(&self, position: &[f32], radius: f32, color: &[f32]) -> Result<(), JsValue> {
        if position.len() < 3 {
            return Err(to_js_error("position requires 3 floats"));
        }
        if color.len() < 4 {
            return Err(to_js_error("color requires 4 floats"));
        }
        let mut position_value = [0.0; 3];
        position_value.copy_from_slice(&position[..3]);
        let mut color_value = [0.0; 4];
        color_value.copy_from_slice(&color[..4]);
        self.state
            .helper
            .set(position_value, radius, color_value)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmBallHelper {
        Self {
            state: self.state.clone(),
        }
    }
}
