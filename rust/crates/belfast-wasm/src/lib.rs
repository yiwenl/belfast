//! WebAssembly facade for the Rust Belfast runtime.

mod axis_helper;
mod bind_group;
mod bindings;
mod camera;
mod compute;
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
mod uniform_block;

pub use axis_helper::WasmAxisHelper;
#[cfg(target_arch = "wasm32")]
pub use bind_group::WasmBindGroup;
pub use camera::{WasmOrthographicCamera, WasmPerspectiveCamera};
pub use compute::WasmCompute;
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
pub use uniform_block::WasmUniformBlock;

use wasm_bindgen::prelude::*;

pub(crate) fn slice_to_vec3(name: &str, value: &[f32]) -> Result<[f32; 3], JsValue> {
    if value.len() < 3 {
        return Err(js_sys::Error::new(&format!("{name} requires 3 floats")).into());
    }
    Ok([value[0], value[1], value[2]])
}

pub(crate) fn to_js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
