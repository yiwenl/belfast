use crate::{BelfastError, BelfastResult, Device};

#[derive(Clone, Debug)]
pub struct ShadowMapOptions {
    pub size: u32,
    pub format: wgpu::TextureFormat,
    pub label: String,
}

impl Default for ShadowMapOptions {
    fn default() -> Self {
        Self {
            size: 1024,
            format: wgpu::TextureFormat::Depth32Float,
            label: "ShadowMap".to_string(),
        }
    }
}

pub struct ShadowMap {
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: u32,
    format: wgpu::TextureFormat,
    label: String,
    _texture: wgpu::Texture,
}

impl ShadowMap {
    pub fn create(device: &Device, options: ShadowMapOptions) -> BelfastResult<Self> {
        let size = options.size.max(1);
        let limit = device.gpu().limits().max_texture_dimension_2d;
        if size > limit {
            return Err(BelfastError::TextureDimensionsExceedLimit {
                width: size,
                height: size,
                limit,
            });
        }

        let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some(&options.label),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: options.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{}View", options.label)),
            ..Default::default()
        });
        let sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{}Sampler", options.label)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::Less),
            ..Default::default()
        });

        Ok(Self {
            view,
            sampler,
            size,
            format: options.format,
            label: options.label,
            _texture: texture,
        })
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn begin_render_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("{}Pass", self.label)),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
    }
}
