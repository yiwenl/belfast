use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::Device;

#[derive(Clone)]
pub struct Buffer {
    gpu: Arc<wgpu::Buffer>,
    size: u64,
}

impl Buffer {
    pub fn create(device: &Device, size: u64, usage: wgpu::BufferUsages, label: &str) -> Self {
        let gpu = device.gpu().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        });
        Self {
            gpu: Arc::new(gpu),
            size,
        }
    }

    pub fn from_data<T: bytemuck::Pod>(
        device: &Device,
        data: &[T],
        usage: wgpu::BufferUsages,
        label: &str,
    ) -> Self {
        let contents = bytemuck::cast_slice(data);
        let gpu = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            });
        Self {
            gpu: Arc::new(gpu),
            size: contents.len() as u64,
        }
    }

    pub fn write<T: bytemuck::Pod>(&self, device: &Device, data: &[T], byte_offset: u64) {
        device
            .queue()
            .write_buffer(&self.gpu, byte_offset, bytemuck::cast_slice(data));
    }

    pub fn gpu(&self) -> &wgpu::Buffer {
        &self.gpu
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

pub struct BufferUsage;

impl BufferUsage {
    pub fn vertex() -> wgpu::BufferUsages {
        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
    }

    pub fn index() -> wgpu::BufferUsages {
        wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
    }

    pub fn uniform() -> wgpu::BufferUsages {
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
    }

    pub fn storage() -> wgpu::BufferUsages {
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
    }

    pub fn vertex_storage() -> wgpu::BufferUsages {
        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
    }
}
