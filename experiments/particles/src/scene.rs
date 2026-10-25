use belfast::{
    AxisHelper, AxisHelperOptions, BallHelper, BallHelperOptions, BelfastResult, BindGroup, Buffer,
    BufferUsage, CameraHelper, CameraHelperOptions, Device, OrbitalControl, OrbitalControlOptions,
    PerspectiveCamera,
};

use crate::input::SkeletonInput;
use crate::{
    model::{depth_view, ModelRenderer, PointLight, DEPTH_FORMAT},
    model_data::ModelData,
    petals::{
        initial_particles, motion_bounds, parse_emitters, PetalSystem, PARTICLE_COUNT,
        PARTICLE_COUNT_OPTIONS,
    },
    ply::GaussianCloud,
    shadow::{fit_shadow_camera, ShadowMap, MAP_SIZE, STRENGTH},
    shadow_debug::ShadowDebug,
    sky::SkyRenderer,
    sorting::{SortCamera, SortGate},
    splat::SplatRenderer,
};
use glam::{Mat4, Vec3};

// The removed tree's position, converted from PLY's Y-down / +Z-forward coordinates.
pub const MAGNOLIA_BASE: Vec3 = Vec3::new(0.05, -0.80, -5.98);
pub const MAGNOLIA_HEIGHT: f32 = 2.8;

// High above the tree and rising petals.
pub const COURTYARD_LIGHT: PointLight = PointLight {
    position: [MAGNOLIA_BASE.x, 25.0, MAGNOLIA_BASE.z],
    color: [1.0, 0.88, 0.66],
    intensity: 600.0,
};
pub const LIGHT_MARKER_RADIUS: f32 = 0.25;

/// The experiment lives here. Add particle buffers, compute and drawing to this type.
pub struct Scene {
    pub camera: PerspectiveCamera,
    pub control: OrbitalControl,
    pub skeleton: SkeletonInput,
    axes: AxisHelper,
    light_marker: BallHelper,
    shadow: ShadowMap,
    shadow_debug: ShadowDebug,
    shadow_camera_helper: CameraHelper,
    pub shadow_debug_rect: [f32; 4],
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    splats: Option<SplatRenderer>,
    model: Option<ModelRenderer>,
    tree_transform: Option<Mat4>,
    tree_bounds: Option<(Vec3, Vec3)>,
    petals: Option<PetalSystem>,
    petal_emitters: Vec<Vec3>,
    depth: wgpu::TextureView,
    depth_size: [u32; 2],
    pub sky: SkyRenderer,
    sort_positions: Vec<[f32; 4]>,
    sort_gate: SortGate,
    pending_sort: Option<SortCamera>,
    pub viewport: [u32; 2],
    pub sort_count: u32,
    pub source_count: u32,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let camera_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ExperimentCameraLayout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ExperimentLayout"),
                    bind_group_layouts: &[Some(&camera_layout)],
                    immediate_size: 0,
                });
        let axes = AxisHelper::new(
            device,
            AxisHelperOptions {
                length: 2.0,
                ..AxisHelperOptions::new(device.format(), &pipeline_layout)
            },
        )?;
        let light_marker = BallHelper::new(
            device,
            BallHelperOptions {
                label: "CourtyardPointLightMarker",
                position: COURTYARD_LIGHT.position,
                radius: LIGHT_MARKER_RADIUS,
                color: [1., 1., 0., 1.],
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                ..BallHelperOptions::new(device.format(), &pipeline_layout)
            },
        )?;
        let camera_buffer = Buffer::create(device, 64, BufferUsage::uniform(), "ExperimentCamera");
        let camera_bind_group = BindGroup::create(
            device,
            &camera_layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.gpu().as_entire_binding(),
            }],
            "ExperimentCameraBindGroup",
        );
        let mut camera = PerspectiveCamera::new(40.0_f32.to_radians(), 1.0, 0.02, 200.0);
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            center: [0.0, 0.0, -1.0],
            radius: 1.0,
            min_radius: 0.05,
            max_radius: 100.0,
            rotate_sensitivity: 0.003,
            ..Default::default()
        })?;
        control.update(0.0, &mut camera);
        let shadow = ShadowMap::new(device, MAP_SIZE, Mat4::IDENTITY, STRENGTH)?;
        let shadow_debug = ShadowDebug::new(device, &shadow.map);
        let shadow_camera_helper = CameraHelper::new(
            device,
            &Mat4::IDENTITY.to_cols_array(),
            CameraHelperOptions {
                label: "MagnoliaShadowCameraHelper",
                color: [0.1, 0.85, 1.0],
                ..CameraHelperOptions::new(device.format(), &pipeline_layout)
            },
        )?;
        Ok(Self {
            camera,
            control,
            skeleton: SkeletonInput::default(),
            axes,
            light_marker,
            shadow,
            shadow_debug,
            shadow_camera_helper,
            shadow_debug_rect: [0.; 4],
            camera_buffer,
            camera_bind_group,
            splats: None,
            model: None,
            tree_transform: None,
            tree_bounds: None,
            petals: None,
            petal_emitters: Vec::new(),
            depth: depth_view(device, [1, 1]),
            depth_size: [1, 1],
            sky: SkyRenderer::new(device),
            sort_positions: Vec::new(),
            sort_gate: SortGate::default(),
            pending_sort: None,
            viewport: [1, 1],
            sort_count: 0,
            source_count: 0,
        })
    }

    pub fn load_ply(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let mut cloud = GaussianCloud::parse(bytes)?;
        let renderer = SplatRenderer::new_with_depth(device, &mut cloud, Some(DEPTH_FORMAT))?;
        let count = renderer.count();
        self.source_count = cloud.source_count;
        self.sort_positions = cloud.sort_positions;
        self.splats = Some(renderer);
        self.sort_gate = SortGate::default();
        self.pending_sort = None;
        self.sort_count = 0;
        Ok(count)
    }

    pub fn load_glb(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let mut data = ModelData::parse(bytes)?;
        let transform = data.placement_transform(MAGNOLIA_BASE, MAGNOLIA_HEIGHT)?;
        data.place(MAGNOLIA_BASE, MAGNOLIA_HEIGHT)?;
        let triangles = data
            .primitives
            .iter()
            .map(|p| p.indices.len() / 3)
            .sum::<usize>();
        let bounds = data.bounds();
        let shadow_camera =
            fit_shadow_camera(Vec3::from(COURTYARD_LIGHT.position), bounds.0, bounds.1)?;
        let model = ModelRenderer::new_shadowed(device, &data, &self.shadow)?;
        model.update_light(device, COURTYARD_LIGHT);
        self.shadow_camera_helper
            .set_matrix(&shadow_camera.to_cols_array())
            .map_err(|e| e.to_string())?;
        self.shadow.update_camera(device, shadow_camera);
        self.model = Some(model);
        self.tree_transform = Some(transform);
        self.tree_bounds = Some(bounds);
        self.petals = None;
        self.petal_emitters.clear();
        Ok(triangles as u32)
    }

    pub fn load_petals(
        &mut self,
        device: &Device,
        glb: &[u8],
        emitters_json: &[u8],
    ) -> Result<u32, String> {
        let transform = self
            .tree_transform
            .ok_or("Load the magnolia branches before its petals")?;
        let model = ModelData::parse(glb)?;
        let emitters = parse_emitters(emitters_json)?;
        let particles = initial_particles(&emitters, transform, PARTICLE_COUNT)?;
        let (min, max) = motion_bounds(&emitters, transform, &model);
        let (tree_min, tree_max) = self.tree_bounds.ok_or("Load the magnolia branches first")?;
        let camera = fit_shadow_camera(
            Vec3::from(COURTYARD_LIGHT.position),
            min.min(tree_min),
            max.max(tree_max),
        )?;
        let petals = PetalSystem::new_shadowed(device, &model, &particles, &self.shadow)?;
        petals.update_light(device, COURTYARD_LIGHT);
        self.shadow_camera_helper
            .set_matrix(&camera.to_cols_array())
            .map_err(|e| e.to_string())?;
        self.shadow.update_camera(device, camera);
        self.petals = Some(petals);
        self.petal_emitters = emitters;
        Ok(PARTICLE_COUNT)
    }

    pub fn set_petal_count(&mut self, device: &Device, count: u32) -> Result<u32, String> {
        if !PARTICLE_COUNT_OPTIONS.contains(&count) {
            return Err("Choose 10,000, 100,000 or 500,000 petals".into());
        }
        let transform = self
            .tree_transform
            .ok_or("Load the magnolia branches first")?;
        let petals = self.petals.as_mut().ok_or("Load the petals first")?;
        if count != petals.count() {
            let particles = initial_particles(&self.petal_emitters, transform, count)?;
            petals.reset_particles(device, &particles)?;
        }
        Ok(petals.count())
    }

    pub fn set_petal_scale(&self, device: &Device, scale: f32) -> Result<f32, String> {
        self.petals
            .as_ref()
            .ok_or("Load the petals first")?
            .set_scale(device, scale)
    }

    pub fn take_sort_positions(&mut self) -> Vec<f32> {
        let positions = std::mem::take(&mut self.sort_positions);
        positions.into_iter().flatten().collect()
    }

    fn sort_camera(&self) -> SortCamera {
        SortCamera::new(
            Mat4::from_cols_array(&self.camera.view_matrix()),
            Mat4::from_cols_array(&self.camera.projection_matrix()),
            self.viewport,
        )
    }

    pub fn request_sort(&mut self) -> Vec<f32> {
        let camera = self.sort_camera();
        if self.splats.is_none()
            || self.pending_sort.is_some()
            || !self.sort_gate.needs_sort(&camera)
        {
            return Vec::new();
        }
        self.pending_sort = Some(camera);
        camera.encode()
    }

    pub fn invalidate_sort(&mut self) {
        // Keep an in-flight request reserved until its reply has been drained.
        self.sort_gate = SortGate::default();
    }

    pub fn discard_sort(&mut self) {
        self.pending_sort = None;
        self.invalidate_sort();
    }

    pub fn apply_sort(
        &mut self,
        device: &Device,
        indices: &[u32],
        did_sort: bool,
    ) -> Result<(), String> {
        let camera = self.pending_sort.ok_or("No Gaussian sort is pending")?;
        self.splats
            .as_mut()
            .ok_or("No Gaussian scene loaded")?
            .set_indices(device, indices)?;
        self.sort_gate.accept(camera);
        self.pending_sort = None;
        self.sort_count += u32::from(did_sort);
        Ok(())
    }

    pub fn visible_count(&self) -> u32 {
        self.splats.as_ref().map_or(0, SplatRenderer::visible)
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        if self.depth_size != self.viewport {
            self.depth = depth_view(device, self.viewport);
            self.depth_size = self.viewport;
        }
        self.control.update(delta_seconds, &mut self.camera);
        self.camera_buffer
            .write(device, &self.camera.view_projection_matrix(), 0);
        if let Some(model) = &self.model {
            model.update_camera(device, &self.camera.view_projection_matrix());
        }
        if let Some(petals) = &mut self.petals {
            petals.update(device, delta_seconds, &self.camera.view_projection_matrix());
        }
        if let Some(splats) = &self.splats {
            splats.update_camera(device, &self.sort_camera());
        }

        self.sky.update(
            device,
            Mat4::from_cols_array(&self.camera.view_matrix()),
            Mat4::from_cols_array(&self.camera.projection_matrix()),
        );

        // Future interaction hook: this is None after tracking is lost or becomes stale.
        // Consume the points here to update attractors or a GPU skeleton buffer.
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&mut self, device: &Device, view: &wgpu::TextureView) {
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ExperimentEncoder"),
            });
        if let Some(petals) = &mut self.petals {
            petals.simulate(&mut encoder);
        }
        if self.model.is_some() || self.petals.is_some() {
            // Both passes use the buffer just written by compute, and the same scale.
            let mut pass = self
                .shadow
                .map
                .begin_depth_pass(&mut encoder, wgpu::LoadOp::Clear(1.));
            if let Some(model) = &self.model {
                model.draw_shadow(&mut pass);
            }
            if let Some(petals) = &self.petals {
                petals.draw_shadow(&mut pass);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CourtyardBackground"),
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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.splats.is_some() || self.model.is_some() {
                self.sky.draw(&mut pass);
            } else {
                self.axes.draw(&mut pass, &self.camera_bind_group);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CourtyardGeometry"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // Opaque branches and petals establish depth; nearer splats blend over them.
            // Splats test their center depth but never write depth (they remain sorted).
            if let Some(model) = &self.model {
                model.draw(&mut pass);
            }
            if let Some(petals) = &self.petals {
                petals.draw(&mut pass);
            }
            if self.model.is_some() || self.splats.is_some() {
                self.light_marker.draw(&mut pass, &self.camera_bind_group);
            }
            if let Some(splats) = &self.splats {
                splats.draw(&mut pass);
            }
        }
        if self.model.is_some() {
            // Debug lines stay readable through the scene and never enter the
            // shadow pass. Use exactly the light VP matrix sampled by PCF.
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ShadowCameraFrustum"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                self.shadow_camera_helper
                    .draw(&mut pass, &self.camera_bind_group);
            }
            self.shadow_debug
                .render(&mut encoder, view, self.viewport, self.shadow_debug_rect);
        }
        device.queue().submit([encoder.finish()]);
    }
}
