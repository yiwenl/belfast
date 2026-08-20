use js_sys::{Array, Object, Reflect};
use wasm_bindgen::{prelude::*, JsCast};

use crate::to_js_error;

#[wasm_bindgen(typescript_custom_section)]
const UNIFORM_BLOCK_TYPES: &str = r#"
export type UniformFieldType =
  | "f32"
  | "u32"
  | "vec2"
  | "vec3"
  | "vec4"
  | "mat3"
  | "mat4";

export type UniformBlockSchema = Record<string, UniformFieldType>;
"#;

#[wasm_bindgen(js_name = UniformBlock)]
pub struct WasmUniformBlock {
    inner: belfast::UniformBlock,
    label: Option<String>,
}

impl WasmUniformBlock {
    pub(crate) fn inner(&self) -> &belfast::UniformBlock {
        &self.inner
    }
}

#[wasm_bindgen(js_class = UniformBlock)]
impl WasmUniformBlock {
    #[wasm_bindgen(js_name = create)]
    pub fn create(
        #[wasm_bindgen(unchecked_param_type = "UniformBlockSchema")] schema: JsValue,
        label: Option<String>,
    ) -> Result<WasmUniformBlock, JsValue> {
        let fields = parse_schema(&schema)?;
        let inner = belfast::UniformBlock::create(fields).map_err(to_js_error)?;
        Ok(Self { inner, label })
    }

    #[wasm_bindgen(js_name = set)]
    pub fn set(
        &mut self,
        name: &str,
        #[wasm_bindgen(unchecked_param_type = "number | ArrayLike<number>")] value: JsValue,
    ) -> Result<(), JsValue> {
        let field_type = self.inner.field_type(name).map_err(to_js_error)?;
        match field_type {
            belfast::UniformFieldType::F32 => {
                let value = value.as_f64().ok_or_else(|| {
                    to_js_error(format!("field \"{name}\" expects a number (f32)"))
                })?;
                self.inner
                    .set_f32(name, value as f32)
                    .map(|_| ())
                    .map_err(to_js_error)
            }
            belfast::UniformFieldType::U32 => {
                let value = parse_u32(name, &value)?;
                self.inner
                    .set_u32(name, value)
                    .map(|_| ())
                    .map_err(to_js_error)
            }
            field_type => {
                let values = parse_float_values(name, &value, value_float_count(field_type))?;
                self.inner
                    .set_f32_slice(name, &values)
                    .map(|_| ())
                    .map_err(to_js_error)
            }
        }
    }

    #[wasm_bindgen(getter, js_name = byteSize)]
    pub fn byte_size(&self) -> usize {
        self.inner.byte_size()
    }

    #[wasm_bindgen(getter, js_name = floatCount)]
    pub fn float_count(&self) -> usize {
        self.inner.float_count()
    }

    #[wasm_bindgen(getter)]
    pub fn label(&self) -> Option<String> {
        self.label.clone()
    }

    #[wasm_bindgen(js_name = getOffset)]
    pub fn get_offset(&self, name: &str) -> Result<usize, JsValue> {
        self.inner.get_offset(name).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = toFloat32Array)]
    pub fn to_float32_array(&self) -> Vec<f32> {
        self.inner.f32_data().to_vec()
    }
}

fn parse_schema(schema: &JsValue) -> Result<Vec<(String, belfast::UniformFieldType)>, JsValue> {
    if Array::is_array(schema) {
        return Err(to_js_error("uniform schema must be an object"));
    }
    let object = schema
        .dyn_ref::<Object>()
        .ok_or_else(|| to_js_error("uniform schema must be an object"))?;
    let keys = Object::keys(object);
    if keys.length() == 0 {
        return Err(to_js_error(
            "uniform schema must contain at least one field",
        ));
    }

    keys.iter()
        .map(|key| {
            let name = key
                .as_string()
                .ok_or_else(|| to_js_error("uniform field names must be strings"))?;
            let field_type = Reflect::get(object, &key)
                .map_err(|_| to_js_error(format!("could not read uniform field \"{name}\"")))?
                .as_string()
                .ok_or_else(|| {
                    to_js_error(format!("uniform field \"{name}\" type must be a string"))
                })?;
            Ok((name.clone(), parse_field_type(&name, &field_type)?))
        })
        .collect()
}

fn parse_field_type(name: &str, field_type: &str) -> Result<belfast::UniformFieldType, JsValue> {
    match field_type {
        "f32" => Ok(belfast::UniformFieldType::F32),
        "u32" => Ok(belfast::UniformFieldType::U32),
        "vec2" | "vec2f" => Ok(belfast::UniformFieldType::Vec2F),
        "vec3" | "vec3f" => Ok(belfast::UniformFieldType::Vec3F),
        "vec4" | "vec4f" => Ok(belfast::UniformFieldType::Vec4F),
        "mat3" => Ok(belfast::UniformFieldType::Mat3x3F),
        "mat4" | "mat4x4f" => Ok(belfast::UniformFieldType::Mat4x4F),
        _ => Err(to_js_error(format!(
            "unsupported uniform field type \"{field_type}\" for \"{name}\""
        ))),
    }
}

fn parse_u32(name: &str, value: &JsValue) -> Result<u32, JsValue> {
    let value = value.as_f64().ok_or_else(|| {
        to_js_error(format!(
            "field \"{name}\" expects a u32 integer between 0 and {}",
            u32::MAX
        ))
    })?;
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 || value > u32::MAX as f64 {
        return Err(to_js_error(format!(
            "field \"{name}\" expects a u32 integer between 0 and {}",
            u32::MAX
        )));
    }
    Ok(value as u32)
}

fn parse_float_values(name: &str, value: &JsValue, expected: usize) -> Result<Vec<f32>, JsValue> {
    if value.as_f64().is_some() || value.is_null() || value.is_undefined() {
        return Err(to_js_error(format!(
            "field \"{name}\" expects {expected} floats"
        )));
    }

    let length = Reflect::get(value, &JsValue::from_str("length"))
        .map_err(|_| to_js_error(format!("field \"{name}\" expects {expected} floats")))?
        .as_f64()
        .filter(|length| length.is_finite() && length.fract() == 0.0 && *length >= 0.0)
        .ok_or_else(|| to_js_error(format!("field \"{name}\" expects {expected} floats")))?;
    if length < expected as f64 {
        return Err(to_js_error(format!(
            "uniform field \"{name}\" requires {expected} floats, got {}",
            length as usize
        )));
    }

    (0..expected)
        .map(|index| {
            Reflect::get(value, &JsValue::from_f64(index as f64))
                .map_err(|_| {
                    to_js_error(format!(
                        "uniform field \"{name}\" value at index {index} is not a number"
                    ))
                })?
                .as_f64()
                .map(|item| item as f32)
                .ok_or_else(|| {
                    to_js_error(format!(
                        "uniform field \"{name}\" value at index {index} is not a number"
                    ))
                })
        })
        .collect()
}

fn value_float_count(field_type: belfast::UniformFieldType) -> usize {
    match field_type {
        belfast::UniformFieldType::F32 | belfast::UniformFieldType::U32 => 1,
        belfast::UniformFieldType::Vec2F => 2,
        belfast::UniformFieldType::Vec3F => 3,
        belfast::UniformFieldType::Vec4F => 4,
        belfast::UniformFieldType::Mat3x3F => 9,
        belfast::UniformFieldType::Mat4x4F => 16,
    }
}
