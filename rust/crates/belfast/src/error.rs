use thiserror::Error;

pub type BelfastResult<T> = Result<T, BelfastError>;

#[derive(Debug, Error)]
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
    #[error("vertex buffer layout must include at least one attribute")]
    EmptyVertexAttributes,
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
    #[error("texture dimensions must be greater than 0, got {width}x{height}")]
    InvalidTextureDimensions { width: u32, height: u32 },
    #[error("RGBA texture data requires {expected} bytes, got {actual}")]
    InvalidTextureDataLength { expected: usize, actual: usize },
}
