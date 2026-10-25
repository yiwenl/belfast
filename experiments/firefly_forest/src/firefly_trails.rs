use belfast::{BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device};

use super::{merged_depth, APPEARANCE_SHADER, DEPTH_FORMAT, STATE_SHADER};
pub(super) const FRAMES: u32 = 256;
const CHUNKS: usize = 4;
const CHUNK_FRAMES: u32 = FRAMES / CHUNKS as u32;
const INDEX_COUNT: u32 = (FRAMES - 1) * 3 * 6;

pub(super) fn record_shader() -> String {
    format!(
        "const CHUNK_FRAMES: u32 = {CHUNK_FRAMES}u;\n{STATE_SHADER}\n{}",
        include_str!("firefly-history.wgsl")
    )
}

pub(super) fn draw_shader(merged: bool) -> String {
    let (texture, component) = if merged {
        ("texture_2d<f32>", ".r")
    } else {
        ("texture_depth_2d", "")
    };
    format!(
        "const TRAIL_FRAMES: u32 = {FRAMES}u;\nconst CHUNK_FRAMES: u32 = {CHUNK_FRAMES}u;\n{APPEARANCE_SHADER}\n\
         @group(1) @binding(0) var occlusion: {texture};\n\
         fn forest_depth(pixel: vec2<i32>) -> f32 {{ return textureLoad(occlusion, pixel, 0){component}; }}\n{}",
        include_str!("firefly-trails.wgsl")
    )
}

pub(super) struct Trails {
    pub(super) history: [Buffer; CHUNKS],
    params: Buffer,
    record_pipeline: Compute,
    record_bindings: [[BindGroup; 2]; CHUNKS],
    pipeline: wgpu::RenderPipeline,
    binding: BindGroup,
    depth_pipeline: wgpu::RenderPipeline,
    depth_binding: BindGroup,
    indices: Buffer,
    occlusion: Option<wgpu::BindGroup>,
    depth: wgpu::TextureView,
    viewport: [u32; 2],
    next_slot: u32,
    recorded_count: u32,
}

impl Trails {
    pub(super) fn new(
        device: &Device,
        states: &[Buffer; 2],
        camera: &Buffer,
        appearance: &wgpu::Buffer,
        capacity: u32,
    ) -> Self {
        // Four 64-frame chunks keep each binding below 128 MiB at 100,000 particles.
        let history = std::array::from_fn(|_| {
            Buffer::create(
                device,
                u64::from(capacity * CHUNK_FRAMES) * 16,
                BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
                "FireflyHistory",
            )
        });
        let params = Buffer::create(device, 16, BufferUsage::uniform(), "FireflyTrailParams");
        let record_pipeline = Compute::new(
            device,
            &record_shader(),
            ComputeOptions::new("RecordFireflyHistory"),
        );
        let record_bindings = std::array::from_fn(|chunk| {
            [0, 1].map(|index| {
                BindGroup::create(
                    device,
                    &record_pipeline.get_bind_group_layout(0),
                    &[&states[index], &history[chunk], &params]
                        .iter()
                        .enumerate()
                        .map(|(binding, buffer)| wgpu::BindGroupEntry {
                            binding: binding as u32,
                            resource: buffer.gpu().as_entire_binding(),
                        })
                        .collect::<Vec<_>>(),
                    "FireflyHistoryBindings",
                )
            })
        });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FireflyTrailShader"),
                source: wgpu::ShaderSource::Wgsl(draw_shader(merged_depth(device)).into()),
            });
        let targets = [Some(wgpu::ColorTargetState {
            format: device.format(),
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let create_pipeline = |depth_only: bool| {
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("FireflyTrails"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: (!depth_only).then_some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &targets,
                    }),
                    primitive: Default::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(true),
                        depth_compare: Some(wgpu::CompareFunction::LessEqual),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let pipeline = create_pipeline(false);
        let depth_pipeline = create_pipeline(true);
        let create_binding = |pipeline: &wgpu::RenderPipeline| {
            BindGroup::create(
                device,
                &pipeline.get_bind_group_layout(0),
                &[
                    history[0].gpu(),
                    camera.gpu(),
                    appearance,
                    params.gpu(),
                    history[1].gpu(),
                    history[2].gpu(),
                    history[3].gpu(),
                ]
                .iter()
                .enumerate()
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
                "FireflyTrailDrawBindings",
            )
        };
        let binding = create_binding(&pipeline);
        let depth_binding = create_binding(&depth_pipeline);
        // Share each triangular ring between its two adjacent segments.
        let mut mesh = Vec::<u16>::with_capacity(INDEX_COUNT as usize);
        for node in 0..FRAMES as u16 - 1 {
            for side in 0..3 {
                let a = node * 3 + side;
                let b = node * 3 + (side + 1) % 3;
                mesh.extend([a, a + 3, b + 3, a, b + 3, b]);
            }
        }
        let indices = Buffer::from_data(device, &mesh, BufferUsage::index(), "FireflyTrailIndices");
        Self {
            history,
            params,
            record_pipeline,
            record_bindings,
            pipeline,
            binding,
            depth_pipeline,
            depth_binding,
            indices,
            occlusion: None,
            depth: trail_depth(device, [1, 1]),
            viewport: [1, 1],
            next_slot: 0,
            recorded_count: 0,
        }
    }

    pub(super) fn reset(&mut self) {
        self.next_slot = 0;
        self.recorded_count = 0;
    }

    pub(super) fn update(&mut self, device: &Device, count: u32, viewport: [u32; 2]) {
        let capacity = (self.history[0].size() / (u64::from(CHUNK_FRAMES) * 16)) as u32;
        self.params.write(
            device,
            &[
                self.next_slot,
                capacity,
                count,
                u32::from(count > self.recorded_count),
            ],
            0,
        );
        if self.viewport != viewport {
            self.depth = trail_depth(device, viewport);
            self.viewport = viewport;
        }
    }

    pub(super) fn record<'a>(
        &'a self,
        pass: &mut wgpu::ComputePass<'a>,
        current: usize,
        count: u32,
    ) {
        let chunk = (self.next_slot / CHUNK_FRAMES) as usize;
        let chunks = if count > self.recorded_count {
            0..CHUNKS
        } else {
            chunk..chunk + 1
        };
        for chunk in chunks {
            self.record_bindings[chunk][current].bind_compute(pass, 0);
            self.record_pipeline
                .dispatch(pass, [count.div_ceil(64), 1, 1]);
        }
    }

    pub(super) fn advance(&mut self, count: u32) {
        self.next_slot = (self.next_slot + 1) % FRAMES;
        self.recorded_count = count;
    }

    pub(super) fn set_occlusion(&mut self, device: &Device, view: &wgpu::TextureView) {
        self.occlusion = Some(device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FireflyTrailOcclusion"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            }],
        }));
    }

    pub(super) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        count: u32,
        max_age: f32,
        fading: bool,
        mut timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let Some(occlusion) = &self.occlusion else {
            return;
        };
        let index_count = (max_age.ceil() as u32 * 3 * 6).min(INDEX_COUNT);
        if fading {
            // Fade the nearest solid surface once, independent of tube/instance order.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FireflyTrailFadeDepth"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    stencil_ops: None,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: timestamps
                    .as_ref()
                    .map(|t| wgpu::RenderPassTimestampWrites {
                        query_set: t.query_set,
                        beginning_of_pass_write_index: t.beginning_of_pass_write_index,
                        end_of_pass_write_index: None,
                    }),
                ..Default::default()
            });
            pass.set_pipeline(&self.depth_pipeline);
            self.depth_binding.bind(&mut pass, 0);
            pass.set_index_buffer(self.indices.gpu().slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..index_count, 0, 0..count);
            if let Some(t) = &mut timestamps {
                t.beginning_of_pass_write_index = None;
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("FireflyTrails"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            // A fresh depth target gives solid trails mutual occlusion without
            // leaving last frame's geometry in the cached forest depth.
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                stencil_ops: None,
                depth_ops: Some(wgpu::Operations {
                    load: if fading {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(1.)
                    },
                    store: wgpu::StoreOp::Discard,
                }),
            }),
            timestamp_writes: timestamps.filter(|t| {
                t.beginning_of_pass_write_index.is_some() || t.end_of_pass_write_index.is_some()
            }),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        self.binding.bind(&mut pass, 0);
        pass.set_bind_group(1, occlusion, &[]);
        pass.set_index_buffer(self.indices.gpu().slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..index_count, 0, 0..count);
    }
}

fn trail_depth(device: &Device, size: [u32; 2]) -> wgpu::TextureView {
    device
        .gpu()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("FireflyTrailDepth"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
