use thiserror::Error;

pub type BelfastResult<T> = Result<T, BelfastError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BelfastError {
    #[error("failed to request WebGPU adapter")]
    AdapterUnavailable,
    #[error("failed to request WebGPU device: {0}")]
    RequestDeviceFailed(#[from] wgpu::RequestDeviceError),
    #[error("mesh vertexCount must be greater than 0")]
    InvalidMeshVertexCount,
    #[error("mesh index count must be greater than 0")]
    InvalidMeshIndexCount,
    #[error("vertex buffer slot {0} is already in use")]
    DuplicateVertexBufferSlot(u32),
    #[error("vertex buffer slot {actual} is not contiguous; expected slot {expected}")]
    NonContiguousVertexBufferSlot { expected: u32, actual: u32 },
    #[error("vertex buffer layout must include at least one attribute")]
    EmptyVertexAttributes,
    #[error("vertex buffer array stride must be greater than zero")]
    InvalidVertexBufferStride,
    #[error("vertex buffer array stride {0} must be a multiple of 4")]
    MisalignedVertexBufferStride(u64),
    #[error(
        "vertex attribute at shader location {shader_location} has misaligned offset {offset}"
    )]
    MisalignedVertexAttributeOffset { shader_location: u32, offset: u64 },
    #[error(
        "vertex attribute at shader location {shader_location} exceeds array stride {array_stride}"
    )]
    VertexAttributeExceedsStride {
        shader_location: u32,
        array_stride: u64,
    },
    #[error("vertex attribute shader location {0} is already in use")]
    DuplicateVertexAttributeLocation(u32),
    #[error("vertex buffer at slot {slot} was created by a different device")]
    VertexBufferDeviceMismatch { slot: u32 },
    #[error("vertex buffer slot {slot} exceeds device limit {limit}")]
    VertexBufferSlotExceedsLimit { slot: u32, limit: u32 },
    #[error("vertex buffer array stride {stride} exceeds device limit {limit}")]
    VertexBufferStrideExceedsLimit { stride: u64, limit: u32 },
    #[error("vertex attribute count {count} exceeds device limit {limit}")]
    VertexAttributeCountExceedsLimit { count: u32, limit: u32 },
    #[error("vertex attribute shader location {location} exceeds device limit {limit}")]
    VertexAttributeLocationExceedsLimit { location: u32, limit: u32 },
    #[error(
        "vertex buffer at slot {slot} requires at least {required} bytes for this mesh, got {actual}"
    )]
    VertexBufferTooSmall {
        slot: u32,
        required: u64,
        actual: u64,
    },
    #[error("vertex buffer byte extent overflowed at slot {0}")]
    VertexBufferExtentOverflow(u32),
    #[error("draw was created by a different device")]
    DrawDeviceMismatch,
    #[error("compute was created by a different device")]
    ComputeDeviceMismatch,
    #[error("mesh contains resources created by a different device")]
    MeshDeviceMismatch,
    #[error("mesh layout is incompatible with this draw")]
    DrawMeshLayoutMismatch,
    #[error("index buffer was created by a different device")]
    IndexBufferDeviceMismatch,
    #[error("index buffer requires at least {required} bytes, got {actual}")]
    IndexBufferTooSmall { required: u64, actual: u64 },
    #[error("unknown uniform field \"{0}\"")]
    UnknownUniformField(String),
    #[error("uniform field \"{name}\" expects {expected}, got {actual}")]
    UniformTypeMismatch {
        name: String,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("uniform field \"{name}\" requires {expected} floats, got {actual}")]
    UniformValueTooShort {
        name: String,
        expected: usize,
        actual: usize,
    },
    #[error("camera aspect must be greater than 0")]
    InvalidCameraAspect,
    #[error("camera field of view must be greater than 0")]
    InvalidCameraFov,
    #[error("orbital control option `{0}` is invalid")]
    InvalidOrbitalControlOption(&'static str),
    #[error("axis length must be finite and greater than 0")]
    InvalidAxisLength,
    #[error("texture dimensions must be greater than 0, got {width}x{height}")]
    InvalidTextureDimensions { width: u32, height: u32 },
    #[error("texture dimensions {width}x{height} exceed device limit {limit}")]
    TextureDimensionsExceedLimit { width: u32, height: u32, limit: u32 },
    #[error("RGBA texture data requires {expected} bytes, got {actual}")]
    InvalidTextureDataLength { expected: usize, actual: usize },
}
