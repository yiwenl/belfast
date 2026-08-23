use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{bindings, slice_to_vec3, to_js_error, WasmDevice};

#[wasm_bindgen(typescript_custom_section)]
const CAMERA_HELPER_OPTION_TYPES: &str = r#"
export interface CameraHelperOptions {
  label?: string;
  color?: [number, number, number] | number[];
  depth?: boolean;
}
"#;

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CameraHelperOptionsInput {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    color: Option<Vec<f32>>,
    #[serde(default)]
    depth: bool,
}

pub(crate) struct CameraHelperState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    helper: belfast::CameraHelper,
}

impl CameraHelperState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn helper(&self) -> &belfast::CameraHelper {
        &self.helper
    }
}

#[wasm_bindgen(js_name = CameraHelper)]
pub struct WasmCameraHelper {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) state: Rc<CameraHelperState>,
}

#[wasm_bindgen(js_class = CameraHelper)]
impl WasmCameraHelper {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        view_proj: &[f32],
        #[wasm_bindgen(unchecked_optional_param_type = "CameraHelperOptions")] options: Option<
            JsValue,
        >,
    ) -> Result<WasmCameraHelper, JsValue> {
        if view_proj.len() < 16 {
            return Err(to_js_error("viewProj requires 16 floats"));
        }
        let mut matrix = [0.0; 16];
        matrix.copy_from_slice(&view_proj[..16]);
        let options: CameraHelperOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let label = options.label.unwrap_or_else(|| "CameraHelper".to_owned());
        let color = match options.color.as_deref() {
            Some(color) => slice_to_vec3("color", color)?,
            None => [1.0, 1.0, 1.0],
        };
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
        let helper = belfast::CameraHelper::new(
            &device.inner,
            &matrix,
            belfast::CameraHelperOptions {
                label: &label,
                color,
                depth_stencil: options.depth.then(bindings::default_depth_state),
                ..belfast::CameraHelperOptions::new(device.inner.format(), &pipeline_layout)
            },
        )
        .map_err(to_js_error)?;

        Ok(Self {
            state: Rc::new(CameraHelperState { helper }),
        })
    }

    #[wasm_bindgen(js_name = setMatrix)]
    pub fn set_matrix(&self, device: &WasmDevice, view_proj: &[f32]) -> Result<(), JsValue> {
        if view_proj.len() < 16 {
            return Err(to_js_error("viewProj requires 16 floats"));
        }
        let mut matrix = [0.0; 16];
        matrix.copy_from_slice(&view_proj[..16]);
        self.state
            .helper
            .set_matrix(&device.inner, &matrix)
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmCameraHelper {
        Self {
            state: self.state.clone(),
        }
    }
}
