use crate::{BelfastError, BelfastResult, Device};

#[derive(Clone, Debug)]
pub struct TextureOptions {
    pub label: String,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
}

impl Default for TextureOptions {
    fn default() -> Self {
        Self {
            label: "Texture".to_string(),
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
        }
    }
}

pub struct Texture {
    _gpu: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl Texture {
    pub fn from_rgba8(
        device: &Device,
        width: u32,
        height: u32,
        data: &[u8],
        options: TextureOptions,
    ) -> BelfastResult<Self> {
        if width == 0 || height == 0 {
            return Err(BelfastError::InvalidTextureDimensions { width, height });
        }

        let expected = width as usize * height as usize * 4;
        if data.len() != expected {
            return Err(BelfastError::InvalidTextureDataLength {
                expected,
                actual: data.len(),
            });
        }

        let gpu = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some(&options.label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: options.format,
            usage: options.usage,
            view_formats: &[],
        });
        device.queue().write_texture(
            wgpu::ImageCopyTexture {
                texture: &gpu,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = gpu.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{}Sampler", options.label)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: options.mag_filter,
            min_filter: options.min_filter,
            ..Default::default()
        });

        Ok(Self {
            _gpu: gpu,
            view,
            sampler,
            width,
            height,
            format: options.format,
        })
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}
