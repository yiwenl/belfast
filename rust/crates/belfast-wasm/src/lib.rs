//! WebAssembly facade for the Rust Belfast runtime.

mod bindings;
mod device;
mod draw;
mod resources;

pub use device::WasmDevice;
pub use draw::WasmDraw;
pub use resources::{WasmBuffer, WasmBufferUsage, WasmMesh};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = UniformBlock)]
pub struct WasmUniformBlock {
    inner: belfast::UniformBlock,
}

#[wasm_bindgen(js_class = UniformBlock)]
impl WasmUniformBlock {
    #[wasm_bindgen(js_name = scene)]
    pub fn scene() -> Result<WasmUniformBlock, JsValue> {
        let inner =
            belfast::UniformBlock::create([("viewProj", belfast::UniformFieldType::Mat4x4F)])
                .map_err(to_js_error)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = byteSize)]
    pub fn byte_size(&self) -> usize {
        self.inner.byte_size()
    }

    #[wasm_bindgen(js_name = floatCount)]
    pub fn float_count(&self) -> usize {
        self.inner.float_count()
    }

    #[wasm_bindgen(js_name = getOffset)]
    pub fn get_offset(&self, name: &str) -> Result<usize, JsValue> {
        self.inner.get_offset(name).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = setF32Slice)]
    pub fn set_f32_slice(&mut self, name: &str, values: &[f32]) -> Result<(), JsValue> {
        self.inner
            .set_f32_slice(name, values)
            .map(|_| ())
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = data)]
    pub fn data(&self) -> Vec<f32> {
        self.inner.f32_data().to_vec()
    }
}

#[wasm_bindgen(js_name = PerspectiveCamera)]
pub struct WasmPerspectiveCamera {
    inner: belfast::PerspectiveCamera,
}

#[wasm_bindgen(js_class = PerspectiveCamera)]
impl WasmPerspectiveCamera {
    #[wasm_bindgen(constructor)]
    pub fn new(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            inner: belfast::PerspectiveCamera::new(fovy_radians, aspect, near, far),
        }
    }

    #[wasm_bindgen(js_name = lookAt)]
    pub fn look_at(&mut self, eye: &[f32], target: &[f32]) -> Result<(), JsValue> {
        self.inner
            .look_at(slice_to_vec3("eye", eye)?, slice_to_vec3("target", target)?);
        Ok(())
    }

    #[wasm_bindgen(js_name = setAspect)]
    pub fn set_aspect(&mut self, aspect: f32) -> Result<(), JsValue> {
        self.inner
            .set_aspect(aspect)
            .map(|_| ())
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = getViewProjectionMatrix)]
    pub fn view_projection_matrix(&self) -> Vec<f32> {
        self.inner.view_projection_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getLookAtTarget)]
    pub fn look_at_target(&self) -> Vec<f32> {
        self.inner.look_at_target().to_vec()
    }
}

#[wasm_bindgen(js_name = OrthographicCamera)]
pub struct WasmOrthographicCamera {
    inner: belfast::OrthographicCamera,
}

#[wasm_bindgen(js_class = OrthographicCamera)]
impl WasmOrthographicCamera {
    #[wasm_bindgen(constructor)]
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self {
            inner: belfast::OrthographicCamera::new(left, right, bottom, top, near, far),
        }
    }

    #[wasm_bindgen(js_name = lookAt)]
    pub fn look_at(&mut self, eye: &[f32], target: &[f32]) -> Result<(), JsValue> {
        self.inner
            .look_at(slice_to_vec3("eye", eye)?, slice_to_vec3("target", target)?);
        Ok(())
    }

    #[wasm_bindgen(js_name = getViewProjectionMatrix)]
    pub fn view_projection_matrix(&self) -> Vec<f32> {
        self.inner.view_projection_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getLookAtTarget)]
    pub fn look_at_target(&self) -> Vec<f32> {
        self.inner.look_at_target().to_vec()
    }
}

fn slice_to_vec3(name: &str, value: &[f32]) -> Result<[f32; 3], JsValue> {
    if value.len() < 3 {
        return Err(js_sys::Error::new(&format!("{name} requires 3 floats")).into());
    }
    Ok([value[0], value[1], value[2]])
}

fn to_js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
