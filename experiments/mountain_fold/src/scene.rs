use crate::input::SkeletonInput;
use belfast::{
    BelfastResult, BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device, OrbitalControl,
    OrbitalControlOptions, OrthographicCamera, PerspectiveCamera,
};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

pub const GRID: u32 = 512;
pub const DROPLETS: u32 = 1024;
pub const SAMPLES: u32 = 512;
pub const MAX_LINES: u32 = 320;
const WORLD: f32 = 12.0;
const MSAA_SAMPLES: u32 = 4;
const SURFACE_VERTICES: u32 = (GRID - 1) * (GRID - 1) * 6;
const BASE_VERTICES: u32 = (GRID - 1) * 4 * 6 + 6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    matrix: [f32; 16],
    eye: [f32; 4],
    light: [f32; 4],
    style: [f32; 4],
    material: [f32; 4],
    grid: [f32; 4],
    screen: [f32; 4],
    counts: [u32; 4],
    line_color: [f32; 4],
    base: [f32; 4],
    stroke: [f32; 4],
    inverse_matrix: [f32; 16],
    paper: [f32; 4],
}

pub struct Scene {
    pub camera: PerspectiveCamera,
    pub control: OrbitalControl,
    pub orthographic: bool,
    pub ortho_bound: f32,
    pub svg_preview: bool,
    paper_size: [f32; 4],
    scene_camera: Option<(OrbitalControl, f32)>,
    pub skeleton: SkeletonInput,
    pub terrain: Buffer,
    pub segments: Buffer,
    droplets: Buffer,
    sim_uniform: Buffer,
    view_uniform: Buffer,
    sim_group: BindGroup,
    simulation: Vec<Compute>,
    view_group: wgpu::BindGroup,
    lines_read_group: wgpu::BindGroup,
    lines_read_layout: wgpu::BindGroupLayout,
    lines_write_layout: wgpu::BindGroupLayout,
    lines_write_group: wgpu::BindGroup,
    lines_compute: Compute,
    mesh_pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    line_depth_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    hatch_pipeline: wgpu::RenderPipeline,
    debug_pipeline: wgpu::RenderPipeline,
    depth_view: wgpu::TextureView,
    msaa_color: wgpu::TextureView,
    msaa_depth: wgpu::TextureView,
    width: u32,
    height: u32,
    pub steps: u32,
    pub paused: bool,
    pub step_limit: u32,
    pub steps_per_frame: u32,
    single_steps: u32,
    pub line_count: u32,
    pub line_mode: u32,
    pub hatch_spacing: f32,
    pub pixel_ratio: f32,
    active_strokes: u32,
    pub layer_spacing: f32,
    pub base_thickness: f32,
    pub show_heightmap: bool,
    pub show_initial: bool,
    pub debug_mode: f32,
    pub debug_gain: f32,
    pub terrain_height: f32,
    pub light_yaw: f32,
    pub light_pitch: f32,
    pub spot_angle: f32,
    pub spot_falloff: f32,
    pub contour_spacing: f32,
    pub contour_strength: f32,
    pub contrast: f32,
    pub line_width: f32,
    pub line_wobble: f32,
    pub wobble_scale: f32,
    line_seed: u32,
    pub line_color: [f32; 4],
    pub background_color: wgpu::Color,
    pub shadows: bool,
}

fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn resource(binding: u32, buffer: &Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.gpu().as_entire_binding(),
    }
}
fn depth_view(device: &Device, width: u32, height: u32, samples: u32) -> wgpu::TextureView {
    device
        .gpu()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("MountainCameraDepth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
fn msaa_color(device: &Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .gpu()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("MountainMultisampleColor"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format: device.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
fn write_group(
    device: &Device,
    layout: &wgpu::BindGroupLayout,
    segments: &Buffer,
    depth: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MountainLineGeneration"),
        layout,
        entries: &[
            resource(0, segments),
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(depth),
            },
        ],
    })
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let terrain = Buffer::create(
            device,
            u64::from(GRID * GRID) * 16,
            BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
            "MountainHeightfield",
        );
        let droplets = Buffer::create(
            device,
            u64::from(DROPLETS) * 64,
            BufferUsage::storage(),
            "MountainDroplets",
        );
        let exchanges = Buffer::create(
            device,
            u64::from(GRID * GRID) * 16,
            BufferUsage::storage(),
            "MountainExchanges",
        );
        let soil_flow = Buffer::create(
            device,
            u64::from(GRID * GRID) * 16,
            BufferUsage::storage(),
            "MountainSoilFlow",
        );
        let segments = Buffer::create(
            device,
            u64::from(MAX_LINES * SAMPLES * 2) * 48,
            BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
            "MountainLineSegments",
        );
        let sim_uniform = Buffer::create(
            device,
            64,
            BufferUsage::uniform(),
            "MountainErosionSettings",
        );
        let view_uniform = Buffer::create(
            device,
            std::mem::size_of::<ViewUniform>() as u64,
            BufferUsage::uniform(),
            "MountainView",
        );
        let sim_layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MountainSimulationLayout"),
                entries: &[
                    buffer_entry(
                        0,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Uniform,
                    ),
                    buffer_entry(
                        1,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: false },
                    ),
                    buffer_entry(
                        2,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: false },
                    ),
                    buffer_entry(
                        3,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: false },
                    ),
                    buffer_entry(
                        4,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: false },
                    ),
                ],
            });
        let sim_pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MountainSimulationPipelineLayout"),
                    bind_group_layouts: &[Some(&sim_layout)],
                    immediate_size: 0,
                });
        let sim_group = BindGroup::create(
            device,
            &sim_layout,
            &[
                resource(0, &sim_uniform),
                resource(1, &terrain),
                resource(2, &droplets),
                resource(3, &exchanges),
                resource(4, &soil_flow),
            ],
            "MountainSimulationGroup",
        );
        let simulation = [
            "clear",
            "request",
            "apply",
            "commit",
            "settle_flow",
            "settle_apply",
        ]
        .into_iter()
        .map(|entry| {
            Compute::new(
                device,
                include_str!("erosion.wgsl"),
                ComputeOptions {
                    label: entry,
                    layout: Some(&sim_pipeline_layout),
                    entry_point: entry,
                },
            )
        })
        .collect();
        let view_layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MountainViewLayout"),
                entries: &[
                    buffer_entry(
                        0,
                        wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Uniform,
                    ),
                    buffer_entry(
                        1,
                        wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        wgpu::BufferBindingType::Storage { read_only: true },
                    ),
                ],
            });
        let view_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MountainViewGroup"),
            layout: &view_layout,
            entries: &[resource(0, &view_uniform), resource(1, &terrain)],
        });
        let lines_read_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MountainLinesReadLayout"),
                    entries: &[buffer_entry(
                        0,
                        wgpu::ShaderStages::VERTEX,
                        wgpu::BufferBindingType::Storage { read_only: true },
                    )],
                });
        let lines_read_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MountainLinesReadGroup"),
            layout: &lines_read_layout,
            entries: &[resource(0, &segments)],
        });
        let lines_write_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MountainLinesWriteLayout"),
                    entries: &[
                        buffer_entry(
                            0,
                            wgpu::ShaderStages::COMPUTE,
                            wgpu::BufferBindingType::Storage { read_only: false },
                        ),
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });
        let msaa_depth = depth_view(device, 2, 1, MSAA_SAMPLES);
        let msaa_color = msaa_color(device, 2, 1);
        let depth_view = depth_view(device, 2, 1, 1);
        let lines_write_group = write_group(device, &lines_write_layout, &segments, &depth_view);
        let lines_pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MountainLinesComputeLayout"),
                    bind_group_layouts: &[Some(&view_layout), Some(&lines_write_layout)],
                    immediate_size: 0,
                });
        let lines_code = format!(
            "{}\n{}",
            include_str!("common.wgsl"),
            include_str!("lines.wgsl")
        );
        let lines_compute = Compute::new(
            device,
            &lines_code,
            ComputeOptions {
                label: "MountainVisibleLines",
                layout: Some(&lines_pipeline_layout),
                entry_point: "cs_main",
            },
        );
        let render_layout = device
            .gpu()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MountainRenderLayout"),
                bind_group_layouts: &[Some(&view_layout), Some(&lines_read_layout)],
                immediate_size: 0,
            });
        let module = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("MountainRenderShader"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{}\n{}",
                        include_str!("common.wgsl"),
                        include_str!("terrain.wgsl")
                    )
                    .into(),
                ),
            });
        let make_pipeline = |label: &'static str,
                             vertex: &'static str,
                             fragment: &'static str,
                             depth: (bool, wgpu::CompareFunction),
                             color: bool,
                             samples: u32| {
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&render_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some(vertex),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: color.then_some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some(fragment),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: device.format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(depth.0),
                        depth_compare: Some(depth.1),
                        stencil: Default::default(),
                        bias: wgpu::DepthBiasState {
                            constant: if depth.0 { 0 } else { -2 },
                            ..Default::default()
                        },
                    }),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..Default::default()
                    },
                    multiview_mask: None,
                    cache: None,
                })
        };
        let mesh_pipeline = make_pipeline(
            "MountainMesh",
            "mesh_vertex",
            "mesh_fragment",
            (true, wgpu::CompareFunction::LessEqual),
            true,
            MSAA_SAMPLES,
        );
        let depth_pipeline = make_pipeline(
            "MountainDepth",
            "mesh_vertex",
            "mesh_fragment",
            (true, wgpu::CompareFunction::LessEqual),
            false,
            1,
        );
        let line_depth_pipeline = make_pipeline(
            "MountainUpperLayerDepth",
            "line_depth_vertex",
            "mesh_fragment",
            (true, wgpu::CompareFunction::LessEqual),
            false,
            1,
        );
        let line_pipeline = make_pipeline(
            "MountainInk",
            "line_vertex",
            "line_fragment",
            (false, wgpu::CompareFunction::LessEqual),
            true,
            MSAA_SAMPLES,
        );
        let hatch_pipeline = make_pipeline(
            "MountainHatchInk",
            "line_vertex",
            "line_fragment",
            (false, wgpu::CompareFunction::Always),
            true,
            MSAA_SAMPLES,
        );
        let debug_pipeline = make_pipeline(
            "MountainHeightmap",
            "debug_vertex",
            "debug_fragment",
            (false, wgpu::CompareFunction::Always),
            true,
            MSAA_SAMPLES,
        );
        let mut camera = PerspectiveCamera::new(42.0_f32.to_radians(), 1.0, 0.1, 100.0);
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            center: [0.0, 0.8, 0.0],
            radius: 30.0,
            min_radius: 6.0,
            max_radius: 40.0,
            rotate_sensitivity: 0.006,
            ..Default::default()
        })?;
        control.snap_yaw(45.64277_f32.to_radians());
        control.snap_pitch(28.363918_f32.to_radians());
        control.update(0.0, &mut camera);
        let scene = Self {
            camera,
            control,
            orthographic: true,
            ortho_bound: 9.0,
            svg_preview: false,
            paper_size: [148.0, 210.0, 10.0, 0.18],
            scene_camera: None,
            skeleton: SkeletonInput::default(),
            terrain,
            segments,
            droplets,
            sim_uniform,
            view_uniform,
            sim_group,
            simulation,
            view_group,
            lines_read_group,
            lines_read_layout,
            lines_write_layout,
            lines_write_group,
            lines_compute,
            mesh_pipeline,
            depth_pipeline,
            line_depth_pipeline,
            line_pipeline,
            hatch_pipeline,
            debug_pipeline,
            depth_view,
            msaa_color,
            msaa_depth,
            width: 2,
            height: 1,
            steps: 0,
            paused: false,
            step_limit: 2400,
            steps_per_frame: 6,
            single_steps: 0,
            line_count: 192,
            line_mode: 0,
            hatch_spacing: 4.0,
            pixel_ratio: 1.0,
            active_strokes: 192,
            layer_spacing: 8.0,
            base_thickness: 0.6,
            show_heightmap: false,
            show_initial: false,
            debug_mode: 0.0,
            debug_gain: 1.6,
            terrain_height: 2.8,
            light_yaw: 74.0,
            light_pitch: 20.0,
            spot_angle: 0.5,
            spot_falloff: 2.0,
            contour_spacing: 0.1,
            contour_strength: 0.2,
            contrast: 1.0,
            line_width: 1.0,
            line_wobble: 0.015,
            wobble_scale: 0.6,
            line_seed: 0,
            line_color: [153.0 / 255.0, 146.0 / 255.0, 138.0 / 255.0, 1.0],
            background_color: wgpu::Color {
                r: 237.0 / 255.0,
                g: 229.0 / 255.0,
                b: 219.0 / 255.0,
                a: 1.0,
            },
            shadows: true,
        };
        scene.set_erosion(
            device,
            &[0.15, 5.0, 0.3, 0.22, 0.015, 4.0, 1.0, 200.0, 0.2, 0.18],
        );
        Ok(scene)
    }

    pub fn reset(&mut self, device: &Device, heights: &[f32], seeds: &[u32]) {
        self.line_seed = seeds[0];
        let data: Vec<[f32; 4]> = heights.iter().map(|h| [*h, *h, 0.0, 0.0]).collect();
        self.terrain.write(device, &data, 0);
        let mut state = vec![[0u32; 16]; DROPLETS as usize];
        for (d, seed) in state.iter_mut().zip(seeds) {
            d[12] = (*seed).max(1);
        }
        self.droplets.write(device, &state, 0);
        self.steps = 0;
        self.single_steps = 0;
    }
    pub fn set_erosion(&self, device: &Device, values: &[f32]) {
        let mut data = [0u32; 16];
        data[0] = GRID;
        data[1] = DROPLETS;
        for (destination, value) in data[4..].iter_mut().zip(values) {
            *destination = value.to_bits();
        }
        self.sim_uniform.write(device, &data, 0);
    }
    pub fn advance(&mut self, steps: u32) {
        self.single_steps = self.single_steps.saturating_add(steps.min(256));
    }
    fn line_view_width(&self) -> f32 {
        self.width as f32
            * if self.svg_preview {
                1.0 - 2.0 * self.paper_size[2] / self.paper_size[0]
            } else {
                1.0
            }
    }
    pub fn segment_count(&self) -> u32 {
        self.active_strokes * SAMPLES * if self.line_mode == 1 { 1 } else { 2 }
    }
    pub fn prepare_lines(&mut self, device: &Device) -> Result<(), &'static str> {
        let strokes = if self.line_mode == 1 {
            (self.line_view_width() / (2.0 * self.hatch_spacing * self.pixel_ratio)).floor() as u32
                * 2
                + 1
        } else {
            self.line_count
        };
        let bytes =
            u64::from(strokes) * u64::from(SAMPLES) * 48 * if self.line_mode == 1 { 1 } else { 2 };
        let limits = device.gpu().limits();
        let limit = u64::from(limits.max_storage_buffer_binding_size).min(limits.max_buffer_size);
        if bytes > limit {
            return Err("Line spacing exceeds GPU capacity. Increase Line spacing px or reduce the canvas size.");
        }
        self.active_strokes = strokes;
        if bytes > self.segments.size() {
            let chunk = u64::from(SAMPLES) * 64 * 48;
            self.segments = Buffer::create(
                device,
                (bytes.div_ceil(chunk) * chunk).min(limit),
                BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
                "MountainLineSegments",
            );
            self.lines_read_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MountainLinesReadGroup"),
                layout: &self.lines_read_layout,
                entries: &[resource(0, &self.segments)],
            });
            self.lines_write_group = write_group(
                device,
                &self.lines_write_layout,
                &self.segments,
                &self.depth_view,
            );
        }
        Ok(())
    }
    pub fn set_svg_preview(&mut self, enabled: bool, paper: [f32; 4]) -> BelfastResult<()> {
        self.paper_size = paper;
        if enabled != self.svg_preview {
            if enabled {
                let old_bound = self.ortho_bound;
                let control = self.paper_camera()?;
                self.scene_camera =
                    Some((std::mem::replace(&mut self.control, control), old_bound));
            } else if let Some((control, bound)) = self.scene_camera.take() {
                self.control = control;
                self.ortho_bound = bound;
            }
            self.svg_preview = enabled;
        }
        self.camera.set_aspect(self.view_aspect())?;
        Ok(())
    }
    fn view_aspect(&self) -> f32 {
        let [width, height, margin, _] = self.paper_size;
        if self.svg_preview {
            (width - 2.0 * margin) / (height - 2.0 * margin)
        } else {
            self.width as f32 / self.height as f32
        }
    }
    fn paper_camera(&mut self) -> BelfastResult<OrbitalControl> {
        let [width, height, margin, _] = self.paper_size;
        let aspect = (width - 2.0 * margin) / (height - 2.0 * margin);
        let radius = (WORLD * WORLD * 0.5 + (self.terrain_height * 0.5).powi(2)).sqrt();
        let angle = ((self.camera.fovy_radians() * 0.5).tan() * aspect.min(1.0)).atan();
        let yaw = self.control.yaw();
        let pitch = self.control.pitch();
        let ground = WORLD * 0.5 * (yaw.sin().abs() + yaw.cos().abs()) * pitch.sin();
        let lift = (self.terrain_height * pitch.cos() - ground).max(0.0) * 0.5;
        // Frame the projected ground and peak, not empty space above the terrain corners.
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            center: [
                -yaw.sin() * pitch.sin() * lift,
                pitch.cos() * lift,
                -yaw.cos() * pitch.sin() * lift,
            ],
            radius: (radius / angle.sin()).clamp(6.0, 200.0),
            min_radius: 6.0,
            max_radius: 200.0,
            rotate_sensitivity: 0.006,
            ..Default::default()
        })?;
        control.snap_yaw(self.control.yaw());
        control.snap_pitch(self.control.pitch());
        self.ortho_bound = (radius / aspect.min(1.0)).clamp(2.0, 30.0);
        Ok(control)
    }
    pub fn center_paper_camera(&mut self) -> BelfastResult<()> {
        self.control = self.paper_camera()?;
        Ok(())
    }
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) -> BelfastResult<()> {
        self.width = width;
        self.height = height;
        self.camera.set_aspect(self.view_aspect())?;
        self.depth_view = depth_view(device, width, height, 1);
        self.msaa_color = msaa_color(device, width, height);
        self.msaa_depth = depth_view(device, width, height, MSAA_SAMPLES);
        self.lines_write_group = write_group(
            device,
            &self.lines_write_layout,
            &self.segments,
            &self.depth_view,
        );
        Ok(())
    }
    pub fn update(&mut self, device: &Device, delta_seconds: f32, _now_ms: f64) {
        self.control.update(delta_seconds, &mut self.camera);
        let yaw = self.light_yaw.to_radians();
        let pitch = self.light_pitch.to_radians();
        let light = [
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
            WORLD,
        ];
        let eye = self.camera.position();
        let far = if self.svg_preview { 1000.0 } else { 100.0 };
        let (matrix, view_eye) = if self.orthographic {
            let bound = self.ortho_bound;
            let horizontal = bound * self.camera.aspect();
            let center = self.camera.look_at_target();
            let mut camera =
                OrthographicCamera::new(-horizontal, horizontal, -bound, bound, 0.1, far);
            camera.look_at(eye, center);
            (
                camera.view_projection_matrix(),
                [
                    eye[0] - center[0],
                    eye[1] - center[1],
                    eye[2] - center[2],
                    0.0,
                ],
            )
        } else {
            (
                (Mat4::perspective_rh(self.camera.fovy_radians(), self.camera.aspect(), 0.1, far)
                    * Mat4::from_cols_array(&self.camera.view_matrix()))
                .to_cols_array(),
                [eye[0], eye[1], eye[2], 1.0],
            )
        };
        let uniform = ViewUniform {
            matrix,
            eye: view_eye,
            light,
            style: [
                self.contrast,
                if self.svg_preview {
                    0.0
                } else {
                    self.layer_spacing
                },
                if self.svg_preview {
                    self.paper_size[3] * self.width as f32 / self.paper_size[0]
                } else {
                    self.line_width
                },
                if self.shadows { 1.0 } else { 0.0 },
            ],
            material: [
                self.spot_angle,
                self.spot_falloff,
                self.contour_spacing,
                self.contour_strength,
            ],
            grid: [
                GRID as f32,
                WORLD / (GRID - 1) as f32,
                self.terrain_height * (GRID - 1) as f32 / WORLD,
                self.debug_gain,
            ],
            screen: [
                self.width as f32,
                self.height as f32,
                if self.show_initial { 1.0 } else { 0.0 },
                self.debug_mode,
            ],
            counts: [self.active_strokes, SAMPLES, self.line_seed, self.line_mode],
            line_color: if self.svg_preview {
                [0.0, 0.0, 0.0, 1.0]
            } else {
                self.line_color
            },
            base: [self.base_thickness, 0.0, 0.0, 0.0],
            stroke: [
                self.line_wobble,
                self.wobble_scale,
                2.0 * self.hatch_spacing * self.pixel_ratio / self.line_view_width(),
                0.0,
            ],
            inverse_matrix: Mat4::from_cols_array(&matrix).inverse().to_cols_array(),
            paper: if self.svg_preview {
                [
                    1.0 - 2.0 * self.paper_size[2] / self.paper_size[0],
                    1.0 - 2.0 * self.paper_size[2] / self.paper_size[1],
                    0.0,
                    0.0,
                ]
            } else {
                [1.0, 1.0, 0.0, 0.0]
            },
        };
        self.view_uniform.write(device, &[uniform], 0);
    }
    pub fn render(&mut self, device: &Device, target: &wgpu::TextureView) {
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MountainFrame"),
            });
        let count = if self.single_steps > 0 {
            let count = self.single_steps.min(16);
            self.single_steps -= count;
            count
        } else if !self.paused {
            self.steps_per_frame
                .min(self.step_limit.saturating_sub(self.steps))
        } else {
            0
        };
        for _ in 0..count {
            for (index, compute) in self.simulation.iter().enumerate() {
                let items = if index == 1 || index == 3 {
                    DROPLETS
                } else {
                    GRID * GRID
                };
                compute.run(
                    &mut encoder,
                    Some(&self.sim_group),
                    [items.div_ceil(64), 1, 1],
                    None,
                );
            }
        }
        self.steps += count;
        let width = self.width as f32;
        let height = self.height as f32;
        let mesh_vertices = SURFACE_VERTICES
            + if self.base_thickness > 0.0 {
                BASE_VERTICES
            } else {
                0
            };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MountainVisibilityDepth"),
                // Keep visibility sampling independent of display antialiasing.
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.view_group, &[]);
            pass.set_bind_group(1, &self.lines_read_group, &[]);
            if !self.svg_preview {
                pass.set_pipeline(&self.depth_pipeline);
                pass.draw(0..mesh_vertices, 0..1);
            }
            // The upper depth surface hides its own back lines, but is never drawn in color.
            pass.set_pipeline(&self.line_depth_pipeline);
            pass.draw(0..SURFACE_VERTICES, 0..1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MountainLineVisibility"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &self.view_group, &[]);
            pass.set_bind_group(1, &self.lines_write_group, &[]);
            self.lines_compute.dispatch(
                &mut pass,
                [(self.active_strokes * SAMPLES).div_ceil(64), 1, 1],
            );
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MountainLayeredScene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_color,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(if self.svg_preview {
                            wgpu::Color::WHITE
                        } else {
                            self.background_color
                        }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.msaa_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.view_group, &[]);
            pass.set_bind_group(1, &self.lines_read_group, &[]);
            if self.svg_preview {
                let x = (width * self.paper_size[2] / self.paper_size[0]).round() as u32;
                let y = (height * self.paper_size[2] / self.paper_size[1]).round() as u32;
                pass.set_scissor_rect(x, y, self.width - 2 * x, self.height - 2 * y);
            } else {
                pass.set_pipeline(&self.mesh_pipeline);
                pass.draw(0..mesh_vertices, 0..1);
            }
            // Hatches already use visible depth texels; interpolated stroke depth can cut gaps.
            pass.set_pipeline(if self.line_mode == 1 {
                &self.hatch_pipeline
            } else {
                &self.line_pipeline
            });
            pass.draw(0..6, 0..self.segment_count());
            if self.show_heightmap && !self.svg_preview {
                let size = (height * 0.22).min(width * 0.2).min(height - 24.0).max(1.0);
                pass.set_viewport(12.0, height - size - 12.0, size, size, 0.0, 1.0);
                pass.set_pipeline(&self.debug_pipeline);
                pass.draw(0..3, 0..1);
            }
        }
        device.queue().submit([encoder.finish()]);
    }
}
