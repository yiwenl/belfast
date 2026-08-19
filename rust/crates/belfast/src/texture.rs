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
    device: Device,
    gpu: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

fn validate_dimensions(width: u32, height: u32, limit: u32) -> BelfastResult<()> {
    if width == 0 || height == 0 {
        return Err(BelfastError::InvalidTextureDimensions { width, height });
    }

    if width > limit || height > limit {
        return Err(BelfastError::TextureDimensionsExceedLimit {
            width,
            height,
            limit,
        });
    }

    Ok(())
}

impl Texture {
    pub fn create_2d(
        device: &Device,
        width: u32,
        height: u32,
        options: TextureOptions,
    ) -> BelfastResult<Self> {
        let limit = device.gpu().limits().max_texture_dimension_2d;
        validate_dimensions(width, height, limit)?;

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
            device: device.clone(),
            gpu,
            view,
            sampler,
            width,
            height,
            format: options.format,
        })
    }

    pub fn from_rgba8(
        device: &Device,
        width: u32,
        height: u32,
        data: &[u8],
        options: TextureOptions,
    ) -> BelfastResult<Self> {
        let limit = device.gpu().limits().max_texture_dimension_2d;
        validate_dimensions(width, height, limit)?;

        let expected = width as usize * height as usize * 4;
        if data.len() != expected {
            return Err(BelfastError::InvalidTextureDataLength {
                expected,
                actual: data.len(),
            });
        }

        let texture = Self::create_2d(device, width, height, options)?;
        texture.device.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture.gpu,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
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

        Ok(texture)
    }

    pub fn gpu(&self) -> &wgpu::Texture {
        &self.gpu
    }

    pub fn device(&self) -> &Device {
        &self.device
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

#[cfg(test)]
mod tests {
    use super::validate_dimensions;
    use crate::BelfastError;

    #[test]
    fn validation_rejects_extreme_dimensions_before_size_calculation() {
        assert!(matches!(
            validate_dimensions(u32::MAX, u32::MAX, 8_192),
            Err(BelfastError::TextureDimensionsExceedLimit {
                width: u32::MAX,
                height: u32::MAX,
                limit: 8_192,
            })
        ));
    }
}
