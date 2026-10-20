use belfast::{BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::sorting::SortCamera;

pub const PARTICLE_COUNT: u32 = 10_000;
const MAX_PARTICLE_COUNT: u32 = 100_000;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub fn merged_depth(device: &Device) -> bool {
    device
        .gpu()
        .features()
        .contains(wgpu::Features::FLOAT32_BLENDABLE)
}

const BODY_RADIUS: f32 = 0.012;
// Covers the largest glow billboard as well as its diagonal.
const VIEW_SPRITE_MARGIN: f32 = 0.1;
const DEFAULT_FLASH_DURATION_SECONDS: f32 = 1.5;
const STATE_SHADER: &str = include_str!("firefly-state.wgsl");
const NOISE_SHADER: &str = include_str!("firefly-noise.wgsl");
const UPDATE_SHADER: &str = include_str!("firefly-update.wgsl");
const DRAW_SHADER: &str = include_str!("firefly-draw.wgsl");

#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
struct Particle {
    position_phase: [f32; 4],
    velocity_frequency: [f32; 4],
    flash: [f32; 4],
}

/// Tuning lives here, in the forest's existing Y-up scene coordinates.
#[derive(Clone, Copy)]
struct Settings {
    min: [f32; 3],
    max: [f32; 3],
    neighbor_radius: f32,
    separation_radius: f32,
    separation: f32,
    cohesion: f32,
    alignment: f32,
    max_speed: f32,
    max_acceleration: f32,
    drag: f32,
    noise_strength: f32,
    phase_coupling: f32,
    phase_jitter: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min: [-10., -0.45, -24.],
            max: [10., 3., 6.],
            neighbor_radius: 0.9,
            separation_radius: 0.22,
            separation: 1.2,
            cohesion: 0.16,
            alignment: 0.8,
            max_speed: 0.32,
            max_acceleration: 0.65,
            drag: 0.18,
            noise_strength: 0.12,
            phase_coupling: 0.35,
            phase_jitter: 0.065,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SimParams {
    min_cell: [f32; 4],
    max_neighbor: [f32; 4],
    flock: [f32; 4],
    motion: [f32; 4],
    noise_phase: [f32; 4],
    clock: [f32; 4],
    controls: [f32; 4],
    grid_count: [u32; 4],
    neighbor_view: [[f32; 4]; 6],
    benchmark: [u32; 4],
}

fn initial_particles(settings: &Settings, count: u32) -> Vec<Particle> {
    seeded_particles(settings, count, 0x6d2b79f5)
}

fn seeded_particles(settings: &Settings, count: u32, seed: u32) -> Vec<Particle> {
    // Deterministic initialization makes tuning and GPU regressions reproducible.
    let mut state = if seed == 0 { 0x6d2b79f5 } else { seed };
    let mut random = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 8) as f32 / 16_777_216.
    };
    (0..count)
        .map(|_| {
            let position: [f32; 3] = std::array::from_fn(|i| {
                let low = settings.min[i] + BODY_RADIUS + 0.1;
                let high = settings.max[i] - BODY_RADIUS - 0.1;
                low + random() * (high - low)
            });
            Particle {
                position_phase: [position[0], position[1], position[2], random()],
                velocity_frequency: [
                    (random() - 0.5) * 0.12,
                    (random() - 0.5) * 0.06,
                    (random() - 0.5) * 0.12,
                    0.16 + random() * 0.12,
                ],
                flash: [
                    DEFAULT_FLASH_DURATION_SECONDS,
                    0.,
                    DEFAULT_FLASH_DURATION_SECONDS,
                    0.,
                ],
            }
        })
        .collect()
}

fn simulation_shader() -> String {
    format!("{STATE_SHADER}\n{NOISE_SHADER}\n{UPDATE_SHADER}")
}
fn draw_shader() -> String {
    format!("{STATE_SHADER}\n{DRAW_SHADER}")
}

pub struct FireflySystem {
    settings: Settings,
    states: [Buffer; 2],
    params: Buffer,
    camera: Buffer,
    appearance: wgpu::Buffer,
    clear: Compute,
    build: Compute,
    simulate_pipeline: Compute,
    compute_bindings: [BindGroup; 2],
    render_pipeline: wgpu::RenderPipeline,
    occlusion_binding: Option<wgpu::BindGroup>,
    draw_bindings: [BindGroup; 2],
    current: usize,
    count: u32,
    capacity: u32,
    grid: [u32; 3],
    time: f32,
    phase_time: f32,
    rates: [f32; 2],
    flash_duration: f32,
    pending_dt: f32,
    view_culling: bool,
}

impl FireflySystem {
    pub fn new(device: &Device) -> Self {
        let settings = Settings::default();
        // Initialize both buffers at capacity so count changes only alter the
        // active dispatch/draw range; newly enabled particles already have state.
        let mut system = Self::with_particles(
            device,
            &initial_particles(&settings, MAX_PARTICLE_COUNT),
            settings,
        );
        system.count = PARTICLE_COUNT;
        system.rates = [0.27, 0.28];
        system
    }

    fn with_particles(device: &Device, particles: &[Particle], settings: Settings) -> Self {
        let count = particles.len() as u32;
        let cell_size = settings.neighbor_radius;
        let grid: [u32; 3] = std::array::from_fn(|i| {
            ((settings.max[i] - settings.min[i]) / cell_size).ceil() as u32
        });
        let states = ["FirefliesA", "FirefliesB"].map(|label| {
            Buffer::from_data(
                device,
                particles,
                BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
                label,
            )
        });
        let params = Buffer::create(
            device,
            std::mem::size_of::<SimParams>() as u64,
            BufferUsage::uniform(),
            "FireflySimulationParams",
        );
        let camera = Buffer::create(device, 128, BufferUsage::uniform(), "FireflyCamera");
        let heads = Buffer::create(
            device,
            u64::from(grid.iter().product::<u32>()) * 4,
            BufferUsage::storage(),
            "FireflyGridHeads",
        );
        let links = Buffer::create(
            device,
            u64::from(count) * 4,
            BufferUsage::storage(),
            "FireflyGridLinks",
        );
        // Explicit shared layout lets all three compute entry points share bindings.
        // Exactly four storage buffers fit WebGPU's downlevel per-stage limit.
        let compute_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FireflyComputeLayout"),
                    entries: &std::array::from_fn::<_, 5, _>(|binding| {
                        wgpu::BindGroupLayoutEntry {
                            binding: binding as u32,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: if binding == 0 {
                                    wgpu::BufferBindingType::Uniform
                                } else {
                                    wgpu::BufferBindingType::Storage {
                                        read_only: binding == 1,
                                    }
                                },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }
                    }),
                });
        let layout = device
            .gpu()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("FireflyComputePipelineLayout"),
                bind_group_layouts: &[Some(&compute_layout)],
                immediate_size: 0,
            });
        let source = simulation_shader();
        let pipeline = |label, entry_point| {
            Compute::new(
                device,
                &source,
                ComputeOptions {
                    label,
                    entry_point,
                    layout: Some(&layout),
                },
            )
        };
        let clear = pipeline("FireflyClearGrid", "clear_grid");
        let build = pipeline("FireflyBuildGrid", "build_grid");
        let simulate_pipeline = pipeline("FireflyFlocking", "update_particles");
        let compute_bindings = [0, 1].map(|read| {
            let buffers = [&params, &states[read], &states[1 - read], &heads, &links];
            BindGroup::create(
                device,
                &compute_layout,
                &buffers
                    .iter()
                    .enumerate()
                    .map(|(binding, buffer)| wgpu::BindGroupEntry {
                        binding: binding as u32,
                        resource: buffer.gpu().as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
                "FireflyComputeBindings",
            )
        });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FireflyGlowShader"),
                source: wgpu::ShaderSource::Wgsl(draw_shader().into()),
            });
        let render_pipeline =
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("FireflyGlow"),
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
                        entry_point: Some(if merged_depth(device) {
                            "fs_occluded"
                        } else {
                            "fs_main"
                        }),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: device.format(),
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::Zero,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: Default::default(),
                    depth_stencil: (!merged_depth(device)).then_some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::LessEqual),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let draw_layout = render_pipeline.get_bind_group_layout(0);
        let appearance = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FireflyAppearance"),
                contents: bytemuck::cast_slice(&[
                    [0.60f32, 1.0, 0.08, 0.18],
                    [1.0, 0.91, 0.14, 0.45],
                    [1.0, 0.61, 0.08, 1.4],
                ]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let draw_bindings = [0, 1].map(|index| {
            BindGroup::create(
                device,
                &draw_layout,
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: states[index].gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: camera.gpu().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: appearance.as_entire_binding(),
                    },
                ],
                "FireflyDrawBindings",
            )
        });
        Self {
            settings,
            states,
            params,
            camera,
            appearance,
            clear,
            build,
            simulate_pipeline,
            compute_bindings,
            render_pipeline,
            occlusion_binding: None,
            draw_bindings,
            current: 0,
            count,
            capacity: count,
            grid,
            time: 0.,
            phase_time: 0.,
            rates: [1., 1.],
            flash_duration: DEFAULT_FLASH_DURATION_SECONDS,
            pending_dt: 0.,
            view_culling: true,
        }
    }

    pub fn set_count(&mut self, count: u32) -> u32 {
        self.count = count
            .clamp(PARTICLE_COUNT, MAX_PARTICLE_COUNT)
            .min(self.capacity);
        self.count
    }

    pub fn set_view_culling(&mut self, enabled: bool) {
        self.view_culling = enabled;
    }

    pub fn reset(&mut self, device: &Device, seed: u32) {
        let particles = seeded_particles(&self.settings, self.capacity, seed);
        for state in &self.states {
            state.write(device, &particles, 0);
        }
        self.current = 0;
        self.time = 0.;
        self.phase_time = 0.;
        self.pending_dt = 0.;
    }

    /// Independent motion/cycle time scales. Active flashes always finish in real time.
    pub fn set_rates(&mut self, movement: f32, blinking: f32) -> Result<(), String> {
        if !movement.is_finite() || !blinking.is_finite() {
            return Err("Firefly speed and frequency must be finite".into());
        }
        self.rates = [movement.clamp(0., 1.), blinking.clamp(0., 1.)];
        Ok(())
    }

    /// Capture this duration on the next flash; active flashes retain their timing.
    pub fn set_flash_duration(&mut self, seconds: f32) -> Result<(), String> {
        if !seconds.is_finite() {
            return Err("Firefly flash duration must be finite".into());
        }
        self.flash_duration = seconds.clamp(0.1, 2.);
        Ok(())
    }

    pub fn update(&mut self, device: &Device, dt: f32, camera: &SortCamera) {
        self.pending_dt = if dt.is_finite() {
            dt.clamp(0., 0.1)
        } else {
            0.
        };
        let s = &self.settings;
        let view_projection = camera.projection * camera.view;
        // WebGPU clip depth is 0..w. Normalize in world space so the guard band
        // remains one neighbor radius wide at every depth and camera FOV.
        let neighbor_view = [
            view_projection.row(3) + view_projection.row(0),
            view_projection.row(3) - view_projection.row(0),
            view_projection.row(3) + view_projection.row(1),
            view_projection.row(3) - view_projection.row(1),
            view_projection.row(2),
            view_projection.row(3) - view_projection.row(2),
        ]
        .map(|plane| {
            let mut plane = plane / plane.truncate().length();
            plane.w += s.neighbor_radius + VIEW_SPRITE_MARGIN;
            plane.to_array()
        });
        self.params.write(
            device,
            &[SimParams {
                min_cell: [s.min[0], s.min[1], s.min[2], s.neighbor_radius],
                max_neighbor: [s.max[0], s.max[1], s.max[2], s.neighbor_radius],
                flock: [s.separation_radius, s.separation, s.cohesion, s.alignment],
                motion: [s.max_speed, s.max_acceleration, s.drag, 0.5],
                noise_phase: [0.65, s.noise_strength, 0.12, s.phase_coupling],
                clock: [self.pending_dt, self.time, BODY_RADIUS, self.phase_time],
                controls: [
                    self.rates[0],
                    self.rates[1],
                    s.phase_jitter,
                    self.flash_duration,
                ],
                grid_count: [self.grid[0], self.grid[1], self.grid[2], self.count],
                neighbor_view,
                benchmark: [u32::from(self.view_culling), 0, 0, 0],
            }],
            0,
        );
        let mut matrices = camera.view.to_cols_array().to_vec();
        matrices.extend(camera.projection.to_cols_array());
        self.camera.write(device, &matrices, 0);
    }

    pub fn simulate(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.simulate_profiled(encoder, None);
    }

    pub fn simulate_profiled(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queries: Option<&wgpu::QuerySet>,
    ) {
        for (index, (pipeline, count, label)) in [
            (
                &self.clear,
                self.grid.iter().product::<u32>(),
                "ClearFireflyGrid",
            ),
            (&self.build, self.count, "BuildFireflyGrid"),
            (&self.simulate_pipeline, self.count, "SimulateFireflies"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: queries.map(|query_set| wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(index as u32 * 2),
                    end_of_pass_write_index: Some(index as u32 * 2 + 1),
                }),
            });
            self.compute_bindings[self.current].bind_compute(&mut pass, 0);
            pipeline.dispatch(&mut pass, [count.div_ceil(64), 1, 1]);
        }
        self.current = 1 - self.current;
        // Noise evolves faster than the slowed simulation at low slider values,
        // while zero still pauses it. Motion and cycle integration keep their rates.
        self.time += self.pending_dt * self.rates[0].sqrt();
        self.phase_time += self.pending_dt * self.rates[1].sqrt();
    }

    pub fn set_appearance(
        &self,
        device: &Device,
        min_peak: f32,
        max_peak: f32,
        warm_share: f32,
        colors: &[f32],
    ) -> Result<(), String> {
        if !(0.0..=3.0).contains(&min_peak)
            || !(min_peak..=3.0).contains(&max_peak)
            || !(0.0..=1.0).contains(&warm_share)
            || colors.len() != 9
            || colors.iter().any(|c| !(0.0..=1.0).contains(c))
        {
            return Err("Invalid firefly appearance settings".into());
        }
        let values = [
            [colors[0], colors[1], colors[2], warm_share],
            [colors[3], colors[4], colors[5], min_peak],
            [colors[6], colors[7], colors[8], max_peak],
        ];
        device
            .queue()
            .write_buffer(&self.appearance, 0, bytemuck::cast_slice(&values));
        Ok(())
    }

    pub fn set_occlusion(&mut self, device: &Device, view: &wgpu::TextureView) {
        if merged_depth(device) {
            self.occlusion_binding =
                Some(device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FireflyOcclusion"),
                    layout: &self.render_pipeline.get_bind_group_layout(1),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    }],
                }));
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // Keep both buffers alive; rendering always uses this frame's output.
        debug_assert!(self.states[self.current].size() > 0);
        pass.set_pipeline(&self.render_pipeline);
        self.draw_bindings[self.current].bind(pass, 0);
        if let Some(binding) = &self.occlusion_binding {
            pass.set_bind_group(1, binding, &[]);
        }
        pass.draw(0..6, 0..self.count);
    }
}

pub fn depth_view(device: &Device, size: [u32; 2]) -> wgpu::TextureView {
    device
        .gpu()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("ForestOcclusionDepth"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if merged_depth(device) {
                wgpu::TextureFormat::R32Float
            } else {
                DEPTH_FORMAT
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

#[cfg(test)]
#[path = "fireflies_tests.rs"]
mod tests;
