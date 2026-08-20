//! WebAssembly facade for the Rust Belfast runtime.

mod bind_group;
mod bindings;
mod camera;
mod device;
mod draw;
#[cfg(any(target_arch = "wasm32", test))]
mod frame;
#[cfg(target_arch = "wasm32")]
mod orbital_control;
#[cfg(any(target_arch = "wasm32", test))]
mod render_target;
mod resources;
mod texture;

#[cfg(target_arch = "wasm32")]
pub use bind_group::WasmBindGroup;
pub use camera::{WasmOrthographicCamera, WasmPerspectiveCamera};
pub use device::WasmDevice;
pub use draw::WasmDraw;
#[cfg(target_arch = "wasm32")]
pub use frame::WasmFrame;
#[cfg(target_arch = "wasm32")]
pub use orbital_control::WasmOrbitalControl;
#[cfg(target_arch = "wasm32")]
pub use render_target::WasmRenderTarget;
pub use resources::{WasmBuffer, WasmBufferUsage, WasmMesh};
#[cfg(target_arch = "wasm32")]
pub use texture::WasmTexture;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = UniformBlock)]
pub struct WasmUniformBlock {
    inner: belfast::UniformBlock,
}

impl WasmUniformBlock {
    pub(crate) fn inner(&self) -> &belfast::UniformBlock {
        &self.inner
    }
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

pub(crate) fn slice_to_vec3(name: &str, value: &[f32]) -> Result<[f32; 3], JsValue> {
    if value.len() < 3 {
        return Err(js_sys::Error::new(&format!("{name} requires 3 floats")).into());
    }
    Ok([value[0], value[1], value[2]])
}

pub(crate) fn to_js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
