//! Rust implementation of the Belfast WebGPU rendering API.

mod bind_group;
mod buffer;
mod camera;
mod compute;
mod controls;
mod device;
mod draw;
mod error;
mod geom;
mod helpers;
mod mesh;
mod render_target;
mod texture;
mod uniform_block;

pub use bind_group::BindGroup;
pub use buffer::{Buffer, BufferUsage};
pub use camera::{OrthographicCamera, PerspectiveCamera};
pub use compute::{Compute, ComputeOptions};
pub use controls::{OrbitalControl, OrbitalControlOptions, OrbitalPointerButton};
pub use device::{Device, DeviceOptions};
pub use draw::{Draw, DrawOptions};
pub use error::{BelfastError, BelfastResult};
pub use geom::{Geom, GeometryData};
pub use helpers::{AxisHelper, AxisHelperOptions};
pub use mesh::{
    Mesh, MeshIndexFormat, MeshLayoutSignature, VertexAttributeDescriptor, VertexBufferBinding,
    VertexBufferLayoutDescriptor,
};
pub use render_target::{RenderPassOptions, RenderPassTarget, RenderTarget, RenderTargetOptions};
pub use texture::{Texture, TextureOptions};
pub use uniform_block::{UniformBlock, UniformFieldType};
