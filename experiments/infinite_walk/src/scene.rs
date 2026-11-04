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
    include_str!("points.wgsl")
);
const SHADOW_SIZE: u32 = 2048;

pub struct Scene {
    pub camera: PerspectiveCamera,
    pub walk: Walk,
    pub skeleton: SkeletonInput,
    pipeline: wgpu::RenderPipeline,
    particle_pipeline: wgpu::RenderPipeline,
    particle_shadow_pipeline: wgpu::RenderPipeline,
    particles: Option<Particles>,
    pub particle_settings: ParticleSettings,
    pub filament_settings: FilamentSettings,
    pub filament_rings: u32,
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
    depth: Option<(wgpu::Texture, wgpu::TextureView)>,
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
                            visibility: wgpu::ShaderStages::VERTEX,
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
        let shadow_preview_pipeline =
            device
                .gpu()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("InfiniteWalkShadowPreview"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_shadow_preview"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_shadow_preview"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: device.format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: Default::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
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
            Buffer::create(device, 224, BufferUsage::uniform(), "InfiniteWalkCamera");
        let camera_bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InfiniteWalkCamera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.gpu().as_entire_binding(),
            }],
        });
        Ok(Self {
            camera: PerspectiveCamera::new(88.0_f32.to_radians(), 1.0, 0.1, 100.0),
            walk: Walk::default(),
            skeleton: SkeletonInput::default(),
            pipeline,
            particle_pipeline,
            particle_shadow_pipeline,
            particles: None,
            particle_settings: ParticleSettings::default(),
            filament_settings: particles::filament_defaults(),
            filament_rings: 12,
            filament_reset_pending: false,
            show_filament_debug: false,
            shadow_pipeline,
            shadow_preview_pipeline,
            shadow_map,
            shadow_binding,
            shadow_strength: 0.9,
            show_shadow_map: true,
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
            depth: None,
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
        let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("InfiniteWalkDepth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        self.depth = Some((texture, view));
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        self.filament_reset_pending |= self.walk.advance(delta_seconds);
        let mut eye = self.walk.position();
        eye[1] = self.camera_y;
        let path_length = DRIFT_X.hypot(LENGTH);
        let target_y = eye[1] + path_length * self.camera_pitch_radians.tan();
        self.camera
            .look_at(eye, [eye[0] + DRIFT_X, target_y, eye[2] - LENGTH]);
        let mut uniform = [0.0_f32; 56];
        let projection = self.camera.projection_matrix();
        uniform[..16].copy_from_slice(&self.camera.view_projection_matrix());
        uniform[16..20].copy_from_slice(&[
            projection[0],
            projection[5],
            self.point_size,
            if device.format().is_srgb() { 1.0 } else { 0.0 },
        ]);
        uniform[20..24].copy_from_slice(&[eye[2], 1.0, LENGTH, DRIFT_X]);
        uniform[24] = if self.white_particles { 1.0 } else { 0.0 };
        uniform[25] = self.copy_offset;
        let forward_scale = self.light_forward / path_length;
        let light_eye = Vec3::new(
            eye[0] + DRIFT_X * forward_scale,
            eye[1] + self.light_height,
            eye[2] - LENGTH * forward_scale,
        );
        uniform[28..32].copy_from_slice(&light_eye.extend(self.light_intensity).to_array());
        let radius = self.light_radius;
        uniform[32..36].copy_from_slice(&[self.light_range, radius, radius * (2.0 / 3.0), 0.025]);
        let light_matrix =
            Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.0, self.light_range)
                * Mat4::look_at_rh(light_eye, light_eye - Vec3::Y, Vec3::Z);
        uniform[36..52].copy_from_slice(&light_matrix.to_cols_array());
        uniform[52..56].copy_from_slice(&[SHADOW_SIZE as f32, 0.00002, self.shadow_strength, 0.0]);
        self.camera_buffer.write(device, &uniform, 0);
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&self, device: &Device, view: &wgpu::TextureView) {
        let Some((_, depth)) = &self.depth else {
            return;
        };
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
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.025,
                            g: 0.025,
                            b: 0.025,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
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
            if self.show_shadow_map {
                let [width, height] = self.viewport.map(|value| value as f32);
                let shortest = width.min(height);
                let margin = (16.0 * self.pixel_scale).min(shortest * 0.05);
                let size = (220.0 * self.pixel_scale).min(shortest - 2.0 * margin);
                pass.set_viewport(margin, height - margin - size, size, size, 0.0, 1.0);
                pass.set_pipeline(&self.shadow_preview_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.shadow_binding, &[]);
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
                std::mem::take(&mut self.filament_reset_pending),
            )?;
        }
        Ok(())
    }
}
