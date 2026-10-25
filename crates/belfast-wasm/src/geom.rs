use wasm_bindgen::prelude::*;

use crate::to_js_error;

#[wasm_bindgen(typescript_custom_section)]
const GEOM_TYPES: &str = r#"
export interface GeometryData {
  positions: Float32Array;
  uvs: Float32Array;
  normals: Float32Array;
  indices: Uint16Array | Uint32Array;
}

export interface CubeOptions {
  size?: number;
}

export interface PlaneOptions {
  width?: number;
  height?: number;
}
"#;

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CubeOptionsInput {
    #[serde(default)]
    size: Option<f32>,
}

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlaneOptionsInput {
    #[serde(default)]
    width: Option<f32>,
    #[serde(default)]
    height: Option<f32>,
}

#[wasm_bindgen(js_name = Geom)]
pub struct WasmGeom;

#[wasm_bindgen(js_class = Geom)]
impl WasmGeom {
    #[wasm_bindgen(unchecked_return_type = "GeometryData")]
    pub fn cube(
        #[wasm_bindgen(unchecked_optional_param_type = "CubeOptions")] options: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let options: CubeOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        geometry_to_js(belfast::Geom::cube(options.size.unwrap_or(1.0)))
    }

    #[wasm_bindgen(unchecked_return_type = "GeometryData")]
    pub fn plane(
        #[wasm_bindgen(unchecked_optional_param_type = "PlaneOptions")] options: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let options: PlaneOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        geometry_to_js(belfast::Geom::plane(
            options.width.unwrap_or(1.0),
            options.height.unwrap_or(1.0),
        ))
    }
}

fn geometry_to_js(geometry: belfast::GeometryData) -> Result<JsValue, JsValue> {
    let vertex_count = geometry.positions.len() / 3;
    let object = js_sys::Object::new();
    set_property(
        &object,
        "positions",
        &js_sys::Float32Array::from(geometry.positions.as_slice()),
    )?;
    set_property(
        &object,
        "uvs",
        &js_sys::Float32Array::from(geometry.uvs.as_slice()),
    )?;
    set_property(
        &object,
        "normals",
        &js_sys::Float32Array::from(geometry.normals.as_slice()),
    )?;
    set_property(
        &object,
        "indices",
        &indices_to_js(&geometry.indices, vertex_count),
    )?;
    Ok(object.into())
}

fn indices_to_js(indices: &[u32], vertex_count: usize) -> JsValue {
    if vertex_count > usize::from(u16::MAX)
        || indices.iter().any(|&index| index > u32::from(u16::MAX))
    {
        js_sys::Uint32Array::from(indices).into()
    } else {
        let short: Vec<u16> = indices.iter().map(|&index| index as u16).collect();
        js_sys::Uint16Array::from(short.as_slice()).into()
    }
}

fn set_property(object: &js_sys::Object, name: &str, value: &JsValue) -> Result<(), JsValue> {
    js_sys::Reflect::set(object, &JsValue::from_str(name), value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PlaneOptionsInput;

    #[test]
    fn plane_defaults_to_unit_xy_geometry() {
        let options = PlaneOptionsInput::default();
        let plane =
            belfast::Geom::plane(options.width.unwrap_or(1.0), options.height.unwrap_or(1.0));
        assert_eq!(
            plane.positions,
            vec![-0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.5, 0.5, 0.0, -0.5, 0.5, 0.0]
        );
        assert_eq!(plane.uvs, vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            plane.normals,
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(plane.indices, vec![0, 1, 2, 0, 2, 3]);
    }
}
