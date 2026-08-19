use crate::Device;

#[derive(Clone, Debug)]
pub struct RenderTargetOptions {
    pub width: u32,
    pub height: u32,
    pub label: String,
    pub format: Option<wgpu::TextureFormat>,
    pub sample_count: u32,
    pub with_depth: bool,
    pub depth_format: wgpu::TextureFormat,
    pub depth_texture_usage: wgpu::TextureUsages,
}

impl Default for RenderTargetOptions {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            label: "RenderTarget".to_string(),
            format: None,
            sample_count: 1,
            with_depth: false,
            depth_format: wgpu::TextureFormat::Depth24Plus,
            depth_texture_usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        }
    }
}

impl RenderTargetOptions {
    pub fn resolved_width(&self) -> u32 {
        self.width.max(1)
    }

    pub fn resolved_height(&self) -> u32 {
        self.height.max(1)
    }

    fn resolved_sample_count(&self) -> u32 {
        self.sample_count.max(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderPassOptions {
    pub clear_color: wgpu::Color,
    pub load_op: wgpu::LoadOp<wgpu::Color>,
    pub store_op: wgpu::StoreOp,
    pub depth_load_op: wgpu::LoadOp<f32>,
    pub depth_store_op: wgpu::StoreOp,
}

impl Default for RenderPassOptions {
    fn default() -> Self {
        let clear_color = wgpu::Color {
            r: 0.05,
            g: 0.05,
            b: 0.08,
            a: 1.0,
        };
        Self {
            clear_color,
            load_op: wgpu::LoadOp::Clear(clear_color),
            store_op: wgpu::StoreOp::Store,
            depth_load_op: wgpu::LoadOp::Clear(1.0),
            depth_store_op: wgpu::StoreOp::Store,
        }
    }
}

pub struct RenderPassTarget<'a> {
    pub color_view: &'a wgpu::TextureView,
    pub depth_view: Option<&'a wgpu::TextureView>,
}

pub struct RenderTarget {
    device: Device,
    label: String,
    format: wgpu::TextureFormat,
    sample_count: u32,
    with_depth: bool,
    depth_format: Option<wgpu::TextureFormat>,
    depth_texture_usage: wgpu::TextureUsages,
    width: u32,
    height: u32,
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    multisampled_color_texture: Option<wgpu::Texture>,
    multisampled_color_view: Option<wgpu::TextureView>,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    sampler: wgpu::Sampler,
}

struct RenderTargetTextures {
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    multisampled_color_texture: Option<wgpu::Texture>,
    multisampled_color_view: Option<wgpu::TextureView>,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
}

struct RenderTargetTextureParams<'a> {
    label: &'a str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
    depth_texture_usage: wgpu::TextureUsages,
}

impl RenderTarget {
    pub fn create(device: &Device, options: RenderTargetOptions) -> Self {
        let width = options.resolved_width();
        let height = options.resolved_height();
        let sample_count = options.resolved_sample_count();
        let format = options.format.unwrap_or_else(|| device.format());
        let label = options.label;
        let sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{label}Sampler")),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let depth_format = options.with_depth.then_some(options.depth_format);
        let textures = Self::create_textures(
            device,
            RenderTargetTextureParams {
                label: &label,
                width,
                height,
                format,
                sample_count,
                depth_format,
                depth_texture_usage: options.depth_texture_usage,
            },
        );

        Self {
            device: device.clone(),
            label,
            format,
            sample_count,
            with_depth: options.with_depth,
            depth_format,
            depth_texture_usage: options.depth_texture_usage,
            width,
            height,
            color_texture: textures.color_texture,
            color_view: textures.color_view,
            multisampled_color_texture: textures.multisampled_color_texture,
            multisampled_color_view: textures.multisampled_color_view,
            depth_texture: textures.depth_texture,
            depth_view: textures.depth_view,
            sampler,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }

    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_view.as_ref()
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn render_pass_target(&self) -> RenderPassTarget<'_> {
        RenderPassTarget {
            color_view: self.color_view(),
            depth_view: self.depth_view(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.recreate_textures();
    }

    pub fn begin_render_pass<'encoder>(
        &'encoder self,
        command_encoder: &'encoder mut wgpu::CommandEncoder,
        options: RenderPassOptions,
    ) -> wgpu::RenderPass<'encoder> {
        let color_view = self
            .multisampled_color_view
            .as_ref()
            .unwrap_or(&self.color_view);
        let color_attachment = Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: self
                .multisampled_color_view
                .as_ref()
                .map(|_| &self.color_view),
            ops: wgpu::Operations {
                load: options.load_op,
                store: options.store_op,
            },
        });
        let color_attachments = [color_attachment];
        let depth_stencil_attachment =
            self.depth_view
                .as_ref()
                .map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: options.depth_load_op,
                        store: options.depth_store_op,
                    }),
                    stencil_ops: None,
                });

        command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("{}RenderPass", self.label)),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
    }

    fn recreate_textures(&mut self) {
        let textures = Self::create_textures(
            &self.device,
            RenderTargetTextureParams {
                label: &self.label,
                width: self.width,
                height: self.height,
                format: self.format,
                sample_count: self.sample_count,
                depth_format: self.with_depth.then_some(
                    self.depth_format
                        .expect("depth format exists when with_depth"),
                ),
                depth_texture_usage: self.depth_texture_usage,
            },
        );

        self.color_texture = textures.color_texture;
        self.color_view = textures.color_view;
        self.multisampled_color_texture = textures.multisampled_color_texture;
        self.multisampled_color_view = textures.multisampled_color_view;
        self.depth_texture = textures.depth_texture;
        self.depth_view = textures.depth_view;
    }

    fn create_textures(
        device: &Device,
        params: RenderTargetTextureParams<'_>,
    ) -> RenderTargetTextures {
        let label = params.label;
        let color_texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label}ColorTexture")),
            size: wgpu::Extent3d {
                width: params.width,
                height: params.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: params.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{label}ColorView")),
            ..Default::default()
        });

        let (multisampled_color_texture, multisampled_color_view) = if params.sample_count > 1 {
            let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("{label}MultisampledColorTexture")),
                size: wgpu::Extent3d {
                    width: params.width,
                    height: params.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: params.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: params.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("{label}MultisampledColorView")),
                ..Default::default()
            });
            (Some(texture), Some(view))
        } else {
            (None, None)
        };

        let (depth_texture, depth_view) = if let Some(depth_format) = params.depth_format {
            let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("{label}DepthTexture")),
                size: wgpu::Extent3d {
                    width: params.width,
                    height: params.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: params.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: depth_format,
                usage: params.depth_texture_usage,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("{label}DepthView")),
                ..Default::default()
            });
            (Some(texture), Some(view))
        } else {
            (None, None)
        };

        RenderTargetTextures {
            color_texture,
            color_view,
            multisampled_color_texture,
            multisampled_color_view,
            depth_texture,
            depth_view,
        }
    }
}
