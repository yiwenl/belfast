use crate::{BelfastError, BelfastResult, Buffer, Device};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshIndexFormat {
    Uint16,
    Uint32,
}

impl From<MeshIndexFormat> for wgpu::IndexFormat {
    fn from(value: MeshIndexFormat) -> Self {
        match value {
            MeshIndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
            MeshIndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
        }
    }
}

impl MeshIndexFormat {
    fn byte_size(self) -> u64 {
        match self {
            Self::Uint16 => 2,
            Self::Uint32 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexAttributeDescriptor {
    pub shader_location: u32,
    pub format: wgpu::VertexFormat,
    pub offset: u64,
}

impl From<VertexAttributeDescriptor> for wgpu::VertexAttribute {
    fn from(value: VertexAttributeDescriptor) -> Self {
        Self {
            format: value.format,
            offset: value.offset,
            shader_location: value.shader_location,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VertexBufferLayoutDescriptor {
    pub array_stride: u64,
    pub attributes: Vec<VertexAttributeDescriptor>,
    pub slot: Option<u32>,
    pub step_mode: Option<wgpu::VertexStepMode>,
}

#[derive(Clone)]
pub struct VertexBufferBinding {
    pub buffer: Buffer,
    pub array_stride: u64,
    pub attributes: Vec<VertexAttributeDescriptor>,
    pub slot: Option<u32>,
    pub step_mode: Option<wgpu::VertexStepMode>,
}

#[derive(Clone)]
struct ResolvedVertexBufferBinding {
    buffer: Option<Buffer>,
    slot: u32,
    array_stride: u64,
    step_mode: wgpu::VertexStepMode,
    attributes: Vec<wgpu::VertexAttribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VertexBufferLayoutSignature {
    slot: u32,
    array_stride: u64,
    step_mode: wgpu::VertexStepMode,
    attributes: Vec<VertexAttributeDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshLayoutSignature(Vec<VertexBufferLayoutSignature>);

pub struct Mesh {
    vertex_count: u32,
    bindings: Vec<ResolvedVertexBufferBinding>,
    device: Option<Device>,
    index_buffer: Option<Buffer>,
    index_count: u32,
    index_format: MeshIndexFormat,
}

impl Mesh {
    pub fn new(vertex_count: u32) -> BelfastResult<Self> {
        if vertex_count == 0 {
            return Err(BelfastError::InvalidMeshVertexCount);
        }
        Ok(Self {
            vertex_count,
            bindings: Vec::new(),
            device: None,
            index_buffer: None,
            index_count: 0,
            index_format: MeshIndexFormat::Uint16,
        })
    }

    pub fn add_vertex_buffer(mut self, binding: VertexBufferBinding) -> BelfastResult<Self> {
        self.push_binding(
            Some(binding.buffer),
            binding.array_stride,
            binding.attributes,
            binding.slot,
            binding.step_mode,
        )?;
        Ok(self)
    }

    pub fn add_vertex_buffer_layout(
        mut self,
        descriptor: VertexBufferLayoutDescriptor,
    ) -> BelfastResult<Self> {
        self.push_binding(
            None,
            descriptor.array_stride,
            descriptor.attributes,
            descriptor.slot,
            descriptor.step_mode,
        )?;
        Ok(self)
    }

    pub fn vertex_layouts(&self) -> Vec<Option<wgpu::VertexBufferLayout<'_>>> {
        self.bindings
            .iter()
            .map(|binding| {
                Some(wgpu::VertexBufferLayout {
                    array_stride: binding.array_stride,
                    step_mode: binding.step_mode,
                    attributes: &binding.attributes,
                })
            })
            .collect()
    }

    pub fn set_index_buffer(
        &mut self,
        buffer: Buffer,
        count: u32,
        format: MeshIndexFormat,
    ) -> BelfastResult<&mut Self> {
        if self
            .device
            .as_ref()
            .is_some_and(|device| !device.is_same(buffer.device()))
        {
            return Err(BelfastError::IndexBufferDeviceMismatch);
        }
        let required = u64::from(count) * format.byte_size();
        if buffer.size() < required {
            return Err(BelfastError::IndexBufferTooSmall {
                required,
                actual: buffer.size(),
            });
        }
        self.set_index_buffer_metadata(count, format)?;
        if self.device.is_none() {
            self.device = Some(buffer.device().clone());
        }
        self.index_buffer = Some(buffer);
        Ok(self)
    }

    pub fn set_index_buffer_metadata(
        &mut self,
        count: u32,
        format: MeshIndexFormat,
    ) -> BelfastResult<&mut Self> {
        if count == 0 {
            return Err(BelfastError::InvalidMeshIndexCount);
        }
        self.index_count = count;
        self.index_format = format;
        Ok(self)
    }

    pub fn bind<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        for binding in &self.bindings {
            if let Some(buffer) = &binding.buffer {
                pass.set_vertex_buffer(binding.slot, buffer.gpu().slice(..));
            }
        }
        if let Some(buffer) = &self.index_buffer {
            pass.set_index_buffer(buffer.gpu().slice(..), self.index_format.into());
        }
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn has_index_buffer(&self) -> bool {
        self.index_count > 0
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn index_format(&self) -> MeshIndexFormat {
        self.index_format
    }

    pub fn device(&self) -> Option<&Device> {
        self.device.as_ref()
    }

    pub fn layout_signature(&self) -> MeshLayoutSignature {
        MeshLayoutSignature(
            self.bindings
                .iter()
                .map(|binding| VertexBufferLayoutSignature {
                    slot: binding.slot,
                    array_stride: binding.array_stride,
                    step_mode: binding.step_mode,
                    attributes: binding
                        .attributes
                        .iter()
                        .map(|attribute| VertexAttributeDescriptor {
                            shader_location: attribute.shader_location,
                            format: attribute.format,
                            offset: attribute.offset,
                        })
                        .collect(),
                })
                .collect(),
        )
    }

    fn push_binding(
        &mut self,
        buffer: Option<Buffer>,
        array_stride: u64,
        attributes: Vec<VertexAttributeDescriptor>,
        slot: Option<u32>,
        step_mode: Option<wgpu::VertexStepMode>,
    ) -> BelfastResult<()> {
        if attributes.is_empty() {
            return Err(BelfastError::EmptyVertexAttributes);
        }
        if array_stride == 0 {
            return Err(BelfastError::InvalidVertexBufferStride);
        }
        if !array_stride.is_multiple_of(4) {
            return Err(BelfastError::MisalignedVertexBufferStride(array_stride));
        }

        let expected_slot = self.bindings.len() as u32;
        let slot = slot.unwrap_or(expected_slot);
        if slot != expected_slot {
            return Err(BelfastError::NonContiguousVertexBufferSlot {
                expected: expected_slot,
                actual: slot,
            });
        }

        for (attribute_index, attribute) in attributes.iter().enumerate() {
            let attribute_alignment = attribute.format.size().min(4);
            if !attribute.offset.is_multiple_of(attribute_alignment) {
                return Err(BelfastError::MisalignedVertexAttributeOffset {
                    shader_location: attribute.shader_location,
                    offset: attribute.offset,
                });
            }
            if attribute
                .offset
                .checked_add(attribute.format.size())
                .is_none_or(|end| end > array_stride)
            {
                return Err(BelfastError::VertexAttributeExceedsStride {
                    shader_location: attribute.shader_location,
                    array_stride,
                });
            }
            let duplicates_current_binding = attributes[..attribute_index]
                .iter()
                .any(|existing| existing.shader_location == attribute.shader_location);
            if duplicates_current_binding
                || self.bindings.iter().any(|binding| {
                    binding
                        .attributes
                        .iter()
                        .any(|existing| existing.shader_location == attribute.shader_location)
                })
            {
                return Err(BelfastError::DuplicateVertexAttributeLocation(
                    attribute.shader_location,
                ));
            }
        }
        let step_mode = step_mode.unwrap_or(wgpu::VertexStepMode::Vertex);
        if let Some(buffer) = buffer.as_ref() {
            self.validate_buffer_binding(buffer, slot, array_stride, &attributes, step_mode)?;
            if self.device.is_none() {
                self.device = Some(buffer.device().clone());
            }
        }

        self.bindings.push(ResolvedVertexBufferBinding {
            buffer,
            slot,
            array_stride,
            step_mode,
            attributes: attributes.into_iter().map(Into::into).collect(),
        });
        Ok(())
    }

    fn validate_buffer_binding(
        &self,
        buffer: &Buffer,
        slot: u32,
        array_stride: u64,
        attributes: &[VertexAttributeDescriptor],
        step_mode: wgpu::VertexStepMode,
    ) -> BelfastResult<()> {
        if self
            .device
            .as_ref()
            .is_some_and(|device| !device.is_same(buffer.device()))
        {
            return Err(BelfastError::VertexBufferDeviceMismatch { slot });
        }

        let limits = buffer.device().gpu().limits();
        if slot >= limits.max_vertex_buffers {
            return Err(BelfastError::VertexBufferSlotExceedsLimit {
                slot,
                limit: limits.max_vertex_buffers,
            });
        }
        if array_stride > u64::from(limits.max_vertex_buffer_array_stride) {
            return Err(BelfastError::VertexBufferStrideExceedsLimit {
                stride: array_stride,
                limit: limits.max_vertex_buffer_array_stride,
            });
        }

        let attribute_count = self
            .bindings
            .iter()
            .map(|binding| binding.attributes.len() as u32)
            .sum::<u32>()
            + attributes.len() as u32;
        if attribute_count > limits.max_vertex_attributes {
            return Err(BelfastError::VertexAttributeCountExceedsLimit {
                count: attribute_count,
                limit: limits.max_vertex_attributes,
            });
        }
        if let Some(attribute) = attributes
            .iter()
            .find(|attribute| attribute.shader_location >= limits.max_vertex_attributes)
        {
            return Err(BelfastError::VertexAttributeLocationExceedsLimit {
                location: attribute.shader_location,
                limit: limits.max_vertex_attributes,
            });
        }

        let element_count = match step_mode {
            wgpu::VertexStepMode::Vertex => u64::from(self.vertex_count),
            wgpu::VertexStepMode::Instance => 1,
        };
        let attribute_end = attributes
            .iter()
            .map(|attribute| attribute.offset + attribute.format.size())
            .max()
            .unwrap_or(0);
        let required = element_count
            .saturating_sub(1)
            .checked_mul(array_stride)
            .and_then(|prefix| prefix.checked_add(attribute_end))
            .ok_or(BelfastError::VertexBufferExtentOverflow(slot))?;
        if buffer.size() < required {
            return Err(BelfastError::VertexBufferTooSmall {
                slot,
                required,
                actual: buffer.size(),
            });
        }

        Ok(())
    }
}
