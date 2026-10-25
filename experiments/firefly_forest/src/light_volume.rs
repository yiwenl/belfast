use belfast::{BindGroup, Compute, ComputeOptions, Device};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::fireflies::{FireflySystem, APPEARANCE_SHADER, STATE_SHADER};

// X/Z/Y: the short vertical axis needs only 32 additive render passes.
const DIMENSIONS: [u32; 3] = [96, 128, 32];
const MAX_RADIUS: f32 = 2.0;
pub(crate) const PARAMS_SHADER: &str = include_str!("light-volume-params.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    min_radius: [f32; 4],
    extent_strength: [f32; 4],
    dimensions_capacity: [u32; 4],
}

pub struct LightVolume {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub params: wgpu::Buffer,
    values: Params,
    pipeline: wgpu::RenderPipeline,
    sources: [wgpu::BindGroup; 2],
    slices: Vec<wgpu::TextureView>,
    resolve: Compute,
    resolve_binding: BindGroup,
    enabled: bool,
    interval: f32,
    elapsed: f32,
    dirty: bool,
}

impl LightVolume {
    pub fn new(device: &Device, fireflies: &FireflySystem) -> Self {
        let (min, max, capacity) = fireflies.light_bounds();
        let values = Params {
            min_radius: [
                min[0] - MAX_RADIUS,
                min[1] - MAX_RADIUS,
                min[2] - MAX_RADIUS,
                0.9,
            ],
            extent_strength: [
                max[0] - min[0] + 2. * MAX_RADIUS,
                max[1] - min[1] + 2. * MAX_RADIUS,
                max[2] - min[2] + 2. * MAX_RADIUS,
                0.0,
            ],
            dimensions_capacity: [DIMENSIONS[0], DIMENSIONS[1], DIMENSIONS[2], capacity],
        };
        let params = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LightVolumeParams"),
                contents: bytemuck::bytes_of(&values),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let view = device
            .gpu()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("FireflyLightVolume"),
                size: wgpu::Extent3d {
                    width: DIMENSIONS[0],
                    height: DIMENSIONS[1],
                    depth_or_array_layers: DIMENSIONS[2],
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        // shortcut: wgpu 30's WebGPU backend drops depth_slice; remove this staging
        // array/resolve when upgrading to a backend that forwards it correctly.
        let layers = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("FireflyLightSlices"),
            size: wgpu::Extent3d {
                width: DIMENSIONS[0],
                height: DIMENSIONS[1],
                depth_or_array_layers: DIMENSIONS[2],
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let slices = (0..DIMENSIONS[2])
            .map(|slice| {
                layers.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: slice,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let resolve = Compute::new(
            device,
            include_str!("light-volume-resolve.wgsl"),
            ComputeOptions::new("LightVolumeResolve"),
        );
        let resolve_binding = BindGroup::create(
            device,
            &resolve.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &layers.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
            "LightVolumeResolveBindings",
        );
        let sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("LightVolumeTrilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FireflyLightVolumeShader"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{STATE_SHADER}\n{APPEARANCE_SHADER}\n{PARAMS_SHADER}\n{}",
                        include_str!("light-volume.wgsl")
                    )
                    .into(),
                ),
            });
        let additive = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("FireflyLightVolumePipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[("peak_brightness", if device.hdr() { 3.0 } else { 1.8 })],
                        ..Default::default()
                    },
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState {
                            color: additive,
                            alpha: additive,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let (states, appearance) = fireflies.light_sources();
        let sources = states.map(|state| {
            device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("LightVolumeSource"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: state.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: appearance.as_entire_binding(),
                    },
                ],
            })
        });
        Self {
            view,
            sampler,
            params,
            values,
            pipeline,
            sources,
            slices,
            resolve,
            resolve_binding,
            enabled: false,
            interval: 1. / 30.,
            elapsed: 0.,
            dirty: true,
        }
    }

    pub fn configure(
        &mut self,
        device: &Device,
        enabled: bool,
        strength: f32,
        radius: f32,
        hz: f32,
    ) -> Result<(), String> {
        if !(0.0..=1.0).contains(&strength)
            || !(0.25..=MAX_RADIUS).contains(&radius)
            || !(hz == 0. || (1.0..=120.0).contains(&hz))
        {
            return Err("Invalid firefly light volume settings".into());
        }
        let interval = if hz == 0. { 0. } else { 1. / hz };
        if self.enabled != enabled
            || self.values.min_radius[3] != radius
            || self.interval != interval
        {
            self.invalidate();
        }
        self.enabled = enabled;
        self.interval = interval;
        self.values.min_radius[3] = radius;
        self.values.extent_strength[3] = if enabled { strength } else { 0. };
        device
            .queue()
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&self.values));
        Ok(())
    }

    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.elapsed = 0.;
    }

    pub fn advance(&mut self, dt: f32) {
        if self.enabled {
            self.elapsed += dt.clamp(0., 0.1);
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        fireflies: &FireflySystem,
        queries: Option<&wgpu::QuerySet>,
    ) -> bool {
        if !self.enabled || (!self.dirty && self.elapsed + 1e-6 < self.interval) {
            return false;
        }
        self.elapsed = if self.dirty || self.interval == 0. {
            0.
        } else {
            (self.elapsed + 1e-6) % self.interval
        };
        self.dirty = false;
        let (current, count) = fireflies.light_state();
        for slice in 0..DIMENSIONS[2] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FireflyLightSlice"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.slices[slice as usize],
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                // Include volume generation in the forest span without GPU readback.
                timestamp_writes: queries.filter(|_| slice == 0).map(|query_set| {
                    wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(8),
                        end_of_pass_write_index: None,
                    }
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.sources[current], &[]);
            let first = slice * self.values.dimensions_capacity[3];
            pass.draw(0..4, first..first + count);
        }
        self.resolve.run(
            encoder,
            Some(&self.resolve_binding),
            [
                DIMENSIONS[0].div_ceil(8),
                DIMENSIONS[1].div_ceil(8),
                DIMENSIONS[2],
            ],
            Some("ResolveFireflyLightVolume"),
        );
        true
    }
}
