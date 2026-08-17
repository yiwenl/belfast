use crate::{BelfastError, BelfastResult, Buffer};

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

pub struct Mesh {
    vertex_count: u32,
    bindings: Vec<ResolvedVertexBufferBinding>,
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
        if self.bindings.is_empty() {
            return Vec::new();
        }
        let max_slot = self
            .bindings
            .iter()
            .map(|binding| binding.slot)
            .max()
            .unwrap_or(0);
        let mut layouts = vec![None; max_slot as usize + 1];

        for binding in &self.bindings {
            layouts[binding.slot as usize] = Some(wgpu::VertexBufferLayout {
                array_stride: binding.array_stride,
                step_mode: binding.step_mode,
                attributes: &binding.attributes,
            });
        }

        layouts
    }

    pub fn set_index_buffer(
        &mut self,
        buffer: Buffer,
        count: u32,
        format: MeshIndexFormat,
    ) -> BelfastResult<&mut Self> {
        self.set_index_buffer_metadata(count, format)?;
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
        let slot = slot.unwrap_or_else(|| self.next_free_slot());
        if self.bindings.iter().any(|entry| entry.slot == slot) {
            return Err(BelfastError::DuplicateVertexBufferSlot(slot));
        }
        self.bindings.push(ResolvedVertexBufferBinding {
            buffer,
            slot,
            array_stride,
            step_mode: step_mode.unwrap_or(wgpu::VertexStepMode::Vertex),
            attributes: attributes.into_iter().map(Into::into).collect(),
        });
        Ok(())
    }

    fn next_free_slot(&self) -> u32 {
        let mut slot = 0;
        while self.bindings.iter().any(|binding| binding.slot == slot) {
            slot += 1;
        }
        slot
    }
}
