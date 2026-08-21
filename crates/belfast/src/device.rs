use std::sync::Arc;

use crate::{BelfastError, BelfastResult};

#[derive(Clone, Debug)]
pub struct DeviceOptions {
    pub power_preference: wgpu::PowerPreference,
    pub format: wgpu::TextureFormat,
}

impl Default for DeviceOptions {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }
}

#[derive(Clone)]
pub struct Device {
    inner: Arc<DeviceInner>,
}

struct DeviceInner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    hdr: bool,
}

impl Device {
    pub async fn create_headless() -> BelfastResult<Self> {
        Self::create_headless_with_options(DeviceOptions::default()).await
    }

    pub async fn create_headless_with_options(options: DeviceOptions) -> BelfastResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|_| BelfastError::AdapterUnavailable)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("BelfastDevice"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;

        Ok(Self::from_wgpu(device, queue, options.format, None))
    }

    pub fn from_wgpu(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        hdr: Option<bool>,
    ) -> Self {
        Self {
            inner: Arc::new(DeviceInner {
                device,
                queue,
                format,
                hdr: hdr.unwrap_or(false),
            }),
        }
    }

    pub fn gpu(&self) -> &wgpu::Device {
        &self.inner.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.inner.queue
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.inner.format
    }

    pub fn hdr(&self) -> bool {
        self.inner.hdr
    }

    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
