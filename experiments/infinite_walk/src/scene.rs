use belfast::{
    filament::FilamentSettings, BelfastResult, Buffer, BufferUsage, Device, PerspectiveCamera,
    Texture, TextureOptions,
};
use glam::{Mat4, Vec3};

use crate::{
    input::SkeletonInput,
    particles::{self, ParticleSettings, Particles},
    point_cloud::{Point, PointCloud},
    walk::{Walk, DRIFT_X, LENGTH},
};

pub const SHADER: &str = concat!(
    include_str!("camera.wgsl"),
    "\n",
    include_str!("points.wgsl"),
    "\n",
    include_str!("fog.wgsl")
);
const SHADOW_SIZE: u32 = 2048;
const LIGHT_TILT_DEGREES: f32 = 15.0;

struct FrameTargets {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    fog_inputs: wgpu::BindGroup,
}

pub struct Scene {
    pub camera: PerspectiveCamera,
    pub walk: Walk,
    pub skeleton: SkeletonInput,
    background: wgpu::Color,
    pipeline: wgpu::RenderPipeline,
    particle_pipeline: wgpu::RenderPipeline,
    particle_shadow_pipeline: wgpu::RenderPipeline,
    particles: Option<Particles>,
    pub particle_settings: ParticleSettings,
    pub filament_settings: FilamentSettings,
    pub filament_rings: u32,
    pub filament_auto_reset: bool,
    pub filament_reset_interval: f32,
    filament_reset_pending: bool,
    pub show_filament_debug: bool,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_preview_pipeline: wgpu::RenderPipeline,
    shadow_map: Texture,
    shadow_binding: wgpu::BindGroup,
    shadow_strength: f32,
    show_shadow_map: bool,
    camera_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    points: Option<Buffer>,
    point_count: u32,
    point_size: f32,
    copy_offset: f32,
    white_particles: bool,
    camera_y: f32,
    camera_pitch_radians: f32,
    light_forward: f32,
    light_height: f32,
    light_intensity: f32,
    light_range: f32,
    light_radius: f32,
    viewport: [u32; 2],
    pixel_scale: f32,
    fog_pipeline: wgpu::RenderPipeline,
    fog_density: f32,
    fog_half_height: f32,
    targets: Option<FrameTargets>,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("InfiniteWalkCameraLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let shadow_map = Texture::create_2d(
            device,
            SHADOW_SIZE,
            SHADOW_SIZE,
            TextureOptions::shadow_map("InfiniteWalkShadow"),
        )?;
        let shadow_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("InfiniteWalkShadowSampling"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
                    ],
                });
        let shadow_binding = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InfiniteWalkShadowSampling"),
            layout: &shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(shadow_map.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(shadow_map.sampler()),
                },
            ],
        });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("InfiniteWalkLayout"),
                    bind_group_layouts: &[Some(&layout), Some(&shadow_layout)],
                    immediate_size: 0,
                });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("InfiniteWalkPoints"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let vertex_buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<Point>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x3],
        })];
        let particle_vertex_buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: particles::STRIDE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Unorm8x4,
                    offset: 32,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 12,
                    shader_location: 2,
                },
            ],
        })];
        let pipeline_options = wgpu::RenderPipelineDescriptor {
            label: Some("InfiniteWalkOpaqueCircles"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: device.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        };
        let pipeline = device.gpu().create_render_pipeline(&pipeline_options);
        let particle_pipeline =
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("WalkParticles"),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_particles"),
                        compilation_options: Default::default(),
                        buffers: &particle_vertex_buffers,
                    },
                    ..pipeline_options
                });
        let fullscreen_pipeline = |name, layout, fragment| {
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(name),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_fullscreen"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some(fragment),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: device.format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let shadow_preview_pipeline = fullscreen_pipeline(
            "InfiniteWalkShadowPreview",
            &pipeline_layout,
            "fs_shadow_preview",
        );
        let fog_inputs = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("InfiniteWalkFogInputs"),
                entries: &[
                    (0, wgpu::TextureSampleType::Float { filterable: false }),
                    (1, wgpu::TextureSampleType::Depth),
                ]
                .map(|(binding, sample_type)| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }),
            });
        let fog_layout = device
            .gpu()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("InfiniteWalkFogLayout"),
                bind_group_layouts: &[Some(&layout), Some(&shadow_layout), Some(&fog_inputs)],
                immediate_size: 0,
            });
        let fog_pipeline = fullscreen_pipeline("InfiniteWalkVolumetricFog", &fog_layout, "fs_fog");
        // The depth pass binds only uniforms, never the shadow texture it is writing.
        let shadow_pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("InfiniteWalkShadowDepthLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let shadow_options = wgpu::RenderPipelineDescriptor {
            label: Some("InfiniteWalkShadowDepth"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_shadow"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 1.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        };
        let shadow_pipeline = device.gpu().create_render_pipeline(&shadow_options);
        let particle_shadow_pipeline =
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("WalkParticleShadows"),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_particle_shadow"),
                        compilation_options: Default::default(),
                        buffers: &particle_vertex_buffers,
                    },
                    ..shadow_options
                });
        let camera_buffer =
            Buffer::create(device, 304, BufferUsage::uniform(), "InfiniteWalkCamera");
        let camera_bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InfiniteWalkCamera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.gpu().as_entire_binding(),
            }],
        });
        // Night fog (#131414). An sRGB render target encodes the clear color on write.
        let [r, g, b] = [0x09_u8, 0x09, 0x08].map(|byte| {
            let value = f64::from(byte) / 255.0;
            if !device.format().is_srgb() {
                value
            } else if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        Ok(Self {
            background: wgpu::Color { r, g, b, a: 1.0 },
            camera: PerspectiveCamera::new(88.0_f32.to_radians(), 1.0, 0.1, 100.0),
            walk: Walk::default(),
            skeleton: SkeletonInput::default(),
            pipeline,
            particle_pipeline,
            particle_shadow_pipeline,
            particles: None,
            particle_settings: ParticleSettings::default(),
            filament_settings: particles::filament_defaults(),
            filament_rings: 6,
            filament_auto_reset: false,
            filament_reset_interval: particles::DEFAULT_FILAMENT_RESET_INTERVAL,
            filament_reset_pending: false,
            show_filament_debug: false,
            shadow_pipeline,
            shadow_preview_pipeline,
            shadow_map,
            shadow_binding,
            shadow_strength: 0.99,
            show_shadow_map: false,
            camera_buffer,
            camera_bind_group,
            points: None,
            point_count: 0,
            point_size: 0.01,
            copy_offset: 0.011,
            white_particles: false,
            camera_y: 0.75,
            camera_pitch_radians: 4.0_f32.to_radians(),
            light_forward: 2.9,
            light_height: 4.2,
            light_intensity: 30.0,
            light_range: 7.6,
            light_radius: 1.45,
            viewport: [0, 0],
            pixel_scale: 1.0,
            fog_pipeline,
            fog_density: 0.22,
            fog_half_height: 0.45,
            targets: None,
        })
    }

    pub fn load_point_cloud(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let cloud = PointCloud::parse(bytes)?;
        let count = cloud.points.len() as u32;
        if u64::from(count) * size_of::<Point>() as u64 > device.gpu().limits().max_buffer_size {
            return Err("Point cloud exceeds this GPU's vertex buffer limit".into());
        }
        let points = Buffer::from_data(
            device,
            &cloud.points,
            BufferUsage::vertex_storage(),
            "InfiniteWalkPoints",
        );
        let particles = Particles::new(device, &points, count, &self.camera_buffer)?;
        self.points = Some(points);
        self.particles = Some(particles);
        self.point_count = count;
        Ok(count)
    }

    pub fn set_point_size(&mut self, size: f32) -> Result<(), String> {
        if !size.is_finite() || !(0.001..=0.05).contains(&size) {
            return Err("Particle size must be between 0.001 and 0.05 world units".into());
        }
        self.point_size = size;
        Ok(())
    }

    pub fn set_copy_offset(&mut self, offset: f32) -> Result<(), String> {
        if !(0.0..=0.25).contains(&offset) {
            return Err("Copy offset must be between 0 and 0.25 world units".into());
        }
        self.copy_offset = offset;
        Ok(())
    }

    pub fn set_camera_y(&mut self, y: f32) -> Result<(), String> {
        if !(0.0..=3.0).contains(&y) {
            return Err("Camera Y must be between 0 and 3 world units".into());
        }
        self.camera_y = y;
        Ok(())
    }

    pub fn set_camera_pitch(&mut self, degrees: f32) -> Result<(), String> {
        if !(-85.0..=85.0).contains(&degrees) {
            return Err("Camera pitch must be between -85 and 85 degrees".into());
        }
        self.camera_pitch_radians = degrees.to_radians();
        Ok(())
    }

    pub fn set_white_particles(&mut self, enabled: bool) {
        self.white_particles = enabled;
    }

    pub fn set_shadow_strength(&mut self, strength: f32) -> Result<(), String> {
        if !(0.0..=1.0).contains(&strength) {
            return Err("Shadow strength must be between 0 and 1".into());
        }
        self.shadow_strength = strength;
        Ok(())
    }

    pub fn set_fog_density(&mut self, density: f32) -> Result<(), String> {
        if !(0.0..=0.5).contains(&density) {
            return Err("Fog density must be between 0 and 0.5".into());
        }
        self.fog_density = density;
        Ok(())
    }

    pub fn set_fog_half_height(&mut self, height: f32) -> Result<(), String> {
        if !(0.05..=3.0).contains(&height) {
            return Err("Fog half-height must be between 0.05 and 3 world units".into());
        }
        self.fog_half_height = height;
        Ok(())
    }

    pub fn set_show_shadow_map(&mut self, enabled: bool) {
        self.show_shadow_map = enabled;
    }

    pub fn set_light(
        &mut self,
        forward: f32,
        height: f32,
        intensity: f32,
        range: f32,
        radius: f32,
    ) -> Result<(), String> {
        if !(0.0..=10.0).contains(&forward)
            || !(0.1..=5.0).contains(&height)
            || !(0.0..=30.0).contains(&intensity)
            || !(0.2..=10.0).contains(&range)
            || !(0.05..=5.0).contains(&radius)
        {
            return Err("Invalid cylinder light offset, height, intensity, range or radius".into());
        }
        self.light_forward = forward;
        self.light_height = height;
        self.light_intensity = intensity;
        self.light_range = range;
        self.light_radius = radius;
        Ok(())
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32, pixel_scale: f32) {
        self.pixel_scale = pixel_scale;
        if self.viewport == [width, height] {
            return;
        }
        self.viewport = [width, height];
        let attachment = |label, format| {
            device
                .gpu()
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let depth = attachment("InfiniteWalkDepth", wgpu::TextureFormat::Depth32Float);
        let color = attachment("InfiniteWalkSceneColor", device.format());
        let fog_inputs = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InfiniteWalkFogInputs"),
            layout: &self.fog_pipeline.get_bind_group_layout(2),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth),
                },
            ],
        });
        self.targets = Some(FrameTargets {
            depth,
            color,
            fog_inputs,
        });
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        self.filament_reset_pending |= self.walk.advance(delta_seconds);
        let mut eye = self.walk.position();
        eye[1] = self.camera_y;
        let path_length = DRIFT_X.hypot(LENGTH);
        let target_y = eye[1] + path_length * self.camera_pitch_radians.tan();
        self.camera
            .look_at(eye, [eye[0] + DRIFT_X, target_y, eye[2] - LENGTH]);
        let mut uniform = [0.0_f32; 76];
        let projection = self.camera.projection_matrix();
        let view_projection = Mat4::from_cols_array(&self.camera.view_projection_matrix());
        uniform[..16].copy_from_slice(&view_projection.to_cols_array());
        uniform[56..72].copy_from_slice(&view_projection.inverse().to_cols_array());
        // Wrap each wind phase in f64 so long-running sessions retain smooth motion.
        for (i, speed) in [-0.08, -0.015, -0.035].into_iter().enumerate() {
            uniform[72 + i] = (now_ms * 0.001 * speed).rem_euclid(16.0) as f32;
        }
        uniform[16..20].copy_from_slice(&[
            projection[0],
            projection[5],
            self.point_size,
            if device.format().is_srgb() { 1.0 } else { 0.0 },
        ]);
        uniform[20..24].copy_from_slice(&[eye[2], 1.0, LENGTH, DRIFT_X]);
        uniform[24] = if self.white_particles { 1.0 } else { 0.0 };
        uniform[25] = self.copy_offset;
        uniform[26] = particles::FOREST_FLOOR_Y;
        uniform[27] = self.fog_half_height.recip();
        let forward_scale = self.light_forward / path_length;
        let floor_target = Vec3::new(
            eye[0] + DRIFT_X * forward_scale,
            particles::FOREST_FLOOR_Y,
            eye[2] - LENGTH * forward_scale,
        );
        let camera_right = Vec3::new(LENGTH, 0.0, DRIFT_X) / path_length;
        // Horizontal shift per unit of height keeps the floor target fixed as the light moves.
        let toward_lamp = Vec3::Y + camera_right * LIGHT_TILT_DEGREES.to_radians().tan();
        let light_eye = floor_target + toward_lamp * (eye[1] + self.light_height - floor_target.y);
        let filament_origin =
            floor_target + toward_lamp * (self.particle_settings.ground_y - floor_target.y);
        uniform[28..32].copy_from_slice(&filament_origin.extend(self.light_intensity).to_array());
        let radius = self.light_radius;
        uniform[32..36].copy_from_slice(&[self.light_range, radius, radius * (2.0 / 3.0), 0.025]);
        let light_matrix =
            Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.0, self.light_range)
                * Mat4::look_to_rh(light_eye, -toward_lamp.normalize(), Vec3::Z);
        uniform[36..52].copy_from_slice(&light_matrix.to_cols_array());
        uniform[52..56].copy_from_slice(&[
            SHADOW_SIZE as f32,
            0.00002,
            self.shadow_strength,
            self.fog_density,
        ]);
        self.camera_buffer.write(device, &uniform, 0);
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&self, device: &Device, view: &wgpu::TextureView) {
        let Some(targets) = &self.targets else {
            return;
        };
        let fog_active = self.fog_density > 0.0 && self.light_intensity > 0.0;
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("InfiniteWalkEncoder"),
            });
        {
            let mut pass = self
                .shadow_map
                .begin_depth_pass(&mut encoder, wgpu::LoadOp::Clear(1.0));
            if self.show_shadow_map || (self.shadow_strength > 0.0 && self.light_intensity > 0.0) {
                if let Some(points) = &self.points {
                    pass.set_pipeline(&self.shadow_pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_vertex_buffer(0, points.gpu().slice(..));
                    pass.draw(0..12, 0..self.point_count);
                }
                if self.particle_settings.visible {
                    if let Some(particles) = &self.particles {
                        pass.set_pipeline(&self.particle_shadow_pipeline);
                        pass.set_bind_group(0, &self.camera_bind_group, &[]);
                        pass.set_vertex_buffer(0, particles.buffer().gpu().slice(..));
                        pass.draw(0..6, 0..particles.count());
                    }
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("InfiniteWalkPoints"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: if fog_active { &targets.color } else { view },
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: if fog_active {
                            wgpu::StoreOp::Store
                        } else {
                            wgpu::StoreOp::Discard
                        },
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(points) = &self.points {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.shadow_binding, &[]);
                pass.set_vertex_buffer(0, points.gpu().slice(..));
                pass.draw(0..12, 0..self.point_count);
            }
            if let Some(particles) = &self.particles {
                if self.particle_settings.visible {
                    pass.set_pipeline(&self.particle_pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_bind_group(1, &self.shadow_binding, &[]);
                    pass.set_vertex_buffer(0, particles.buffer().gpu().slice(..));
                    pass.draw(0..6, 0..particles.count());
                }
                if self.show_filament_debug {
                    particles.draw_debug(&mut pass);
                }
            }
        }
        if fog_active || self.show_shadow_map {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("InfiniteWalkComposite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if fog_active {
                            wgpu::LoadOp::Clear(self.background)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(1, &self.shadow_binding, &[]);
            if fog_active {
                pass.set_pipeline(&self.fog_pipeline);
                pass.set_bind_group(2, &targets.fog_inputs, &[]);
                pass.draw(0..3, 0..1);
            }
            if self.show_shadow_map {
                let [width, height] = self.viewport.map(|value| value as f32);
                let shortest = width.min(height);
                let margin = (16.0 * self.pixel_scale).min(shortest * 0.05);
                let size = (220.0 * self.pixel_scale).min(shortest - 2.0 * margin);
                pass.set_viewport(margin, height - margin - size, size, size, 0.0, 1.0);
                pass.set_pipeline(&self.shadow_preview_pipeline);
                pass.draw(0..3, 0..1);
            }
        }
        device.queue().submit([encoder.finish()]);
    }

    pub fn simulate(&mut self, device: &Device, delta_seconds: f32) -> Result<(), String> {
        if let Some(particles) = &mut self.particles {
            particles.simulate(
                device,
                delta_seconds,
                self.particle_settings,
                self.filament_settings,
                self.filament_rings,
                self.filament_auto_reset,
                self.filament_reset_interval,
                std::mem::take(&mut self.filament_reset_pending),
            )?;
        }
        Ok(())
    }
}
