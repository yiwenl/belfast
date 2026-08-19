use wasm_bindgen::prelude::*;

use crate::{bindings, to_js_error, WasmDevice};

#[wasm_bindgen(js_name = BufferUsage)]
pub struct WasmBufferUsage;

#[wasm_bindgen(js_class = BufferUsage)]
impl WasmBufferUsage {
    #[allow(unused_variables)]
    #[wasm_bindgen(getter, static_method_of = WasmBufferUsage)]
    pub fn vertex() -> String {
        "vertex".into()
    }
}

#[wasm_bindgen(js_name = Buffer)]
pub struct WasmBuffer {
    inner: belfast::Buffer,
}

impl WasmBuffer {
    pub(crate) fn inner(&self) -> &belfast::Buffer {
        &self.inner
    }
}

#[wasm_bindgen(js_class = Buffer)]
impl WasmBuffer {
    #[wasm_bindgen(js_name = fromData)]
    pub fn from_data(
        device: &WasmDevice,
        values: &[f32],
        usage: &str,
        label: Option<String>,
    ) -> Result<WasmBuffer, JsValue> {
        if values.is_empty() {
            return Err(to_js_error("buffer values must not be empty"));
        }

        let usage = bindings::parse_buffer_usage(usage).map_err(to_js_error)?;
        let label = label.as_deref().unwrap_or("Buffer");
        Ok(Self {
            inner: belfast::Buffer::from_data(&device.inner, values, usage, label),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize {
        self.inner.size() as usize
    }
}

#[wasm_bindgen(js_name = Mesh)]
pub struct WasmMesh {
    inner: belfast::Mesh,
}

impl WasmMesh {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &belfast::Mesh {
        &self.inner
    }

    pub(crate) fn into_inner(self) -> belfast::Mesh {
        self.inner
    }
}

#[wasm_bindgen(js_class = Mesh)]
impl WasmMesh {
    #[wasm_bindgen(constructor)]
    pub fn new(vertex_count: u32) -> Result<WasmMesh, JsValue> {
        Ok(Self {
            inner: belfast::Mesh::new(vertex_count).map_err(to_js_error)?,
        })
    }

    #[wasm_bindgen(js_name = addVertexBuffer)]
    pub fn add_vertex_buffer(
        self,
        buffer: &WasmBuffer,
        descriptor: JsValue,
    ) -> Result<WasmMesh, JsValue> {
        let descriptor: bindings::VertexBufferDescriptorInput =
            serde_wasm_bindgen::from_value(descriptor).map_err(to_js_error)?;
        let converted = descriptor.try_into_binding().map_err(to_js_error)?;
        let inner = self
            .inner
            .add_vertex_buffer(belfast::VertexBufferBinding {
                buffer: buffer.inner().clone(),
                array_stride: converted.array_stride,
                attributes: converted.attributes,
                slot: converted.slot,
                step_mode: converted.step_mode,
            })
            .map_err(to_js_error)?;

        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = vertexCount)]
    pub fn vertex_count(&self) -> u32 {
        self.inner.vertex_count()
    }

    #[wasm_bindgen(js_name = hasIndexBuffer)]
    pub fn has_index_buffer(&self) -> bool {
        self.inner.has_index_buffer()
    }
}
