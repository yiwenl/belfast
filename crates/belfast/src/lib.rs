//! Rust implementation of the Belfast WebGPU rendering API.

mod buffer;
mod camera;
mod device;
mod draw;
mod error;
mod geom;
mod mesh;
mod uniform_block;

pub use buffer::{Buffer, BufferUsage};
pub use camera::{OrthographicCamera, PerspectiveCamera};
pub use device::{Device, DeviceOptions};
pub use draw::{Draw, DrawOptions};
pub use error::{BelfastError, BelfastResult};
pub use geom::{Geom, GeometryData};
pub use mesh::{
    Mesh, MeshIndexFormat, VertexAttributeDescriptor, VertexBufferBinding,
    VertexBufferLayoutDescriptor,
};
pub use uniform_block::{UniformBlock, UniformFieldType};
