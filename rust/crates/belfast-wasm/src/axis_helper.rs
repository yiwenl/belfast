use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice};

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AxisHelperOptionsInput {
    #[serde(default)]
    length: Option<f32>,
    #[serde(default)]
    label: Option<String>,
}

pub(crate) struct AxisHelperState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    helper: belfast::AxisHelper,
}

impl AxisHelperState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn helper(&self) -> &belfast::AxisHelper {
        &self.helper
    }
}

#[wasm_bindgen(js_name = AxisHelper)]
pub struct WasmAxisHelper {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) state: Rc<AxisHelperState>,
}

#[wasm_bindgen(js_class = AxisHelper)]
impl WasmAxisHelper {
    #[wasm_bindgen(constructor)]
    pub fn new(device: &WasmDevice, options: Option<JsValue>) -> Result<WasmAxisHelper, JsValue> {
        let options: AxisHelperOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let label = options.label.unwrap_or_else(|| "AxisHelper".to_owned());
        let length = options.length.unwrap_or(1.0);
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
        let helper = belfast::AxisHelper::new(
            &device.inner,
            belfast::AxisHelperOptions {
                label: &label,
                length,
                ..belfast::AxisHelperOptions::new(device.inner.format(), &pipeline_layout)
            },
        )
        .map_err(to_js_error)?;

        Ok(Self {
            state: Rc::new(AxisHelperState { helper }),
        })
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmAxisHelper {
        Self {
            state: self.state.clone(),
        }
    }
}
