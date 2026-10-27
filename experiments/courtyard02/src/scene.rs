use crate::{
    markers::FlowerMarkers,
    petal::{depth_view, Petal, DEPTH_FORMAT, MAX_PETALS, MIN_PETALS},
    ply::GaussianCloud,
    shadow::{ShadowMap, MAP_SIZE, STRENGTH},
    shadow_debug::ShadowDebug,
    sorting::{SortCamera, SortGate},
    splat::SplatRenderer,
    vortex::MotionSettings,
};
use belfast::{BelfastResult, Device, OrbitalControl, OrbitalControlOptions, PerspectiveCamera};
use glam::{Mat4, Vec3};

// court.blend / Camera / frame 32: 35 mm lens, 36 mm sensor, AUTO sensor fit.
fn blender_fovy(aspect: f32) -> f32 {
    2.0 * (36.0 / (2.0 * 35.0 * aspect.max(1.0))).atan()
}

pub struct Scene {
    pub camera: PerspectiveCamera,
    pub control: OrbitalControl,
    splats: Option<SplatRenderer>,
    petal: Option<Petal>,
    depth: wgpu::TextureView,
    shadow: ShadowMap,
    shadow_debug: ShadowDebug,
    pub show_shadow_camera: bool,
    pub show_petal: bool,
    pub petal_size: f32,
    petal_count: u32,
    petal_motion: MotionSettings,
    pub flowers: Option<FlowerMarkers>,
    pub show_flower_markers: bool,
    pub splat_hide_distance: f32,
    sort_positions: Vec<[f32; 4]>,
    sort_gate: SortGate,
    pending_sort: Option<SortCamera>,
    pub viewport: [u32; 2],
    pub sort_count: u32,
    pub source_count: u32,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let aspect = 1920.0 / 1080.0;
        let mut camera = PerspectiveCamera::new(blender_fovy(aspect), aspect, 0.1, 100.0);
        // Same Blender-to-scene transform as flower-emitters.json; directions omit the offset.
        let eye = Vec3::new(-2.2536968, 0.15622632, 1.7343298);
        let forward = Vec3::new(0.5391256, 0.0976027, -0.8365508).normalize();
        // Put the orbit pivot at the flower cloud's depth along the camera's viewing ray.
        let radius = 4.1207833;
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            center: (eye + forward * radius).to_array(),
            radius,
            min_radius: 0.05,
            max_radius: 100.0,
            rotate_sensitivity: 0.003,
            ..Default::default()
        })?;
        control.snap_yaw((-forward.x).atan2(-forward.z));
        control.snap_pitch((-forward.y).asin());
        control.update(0.0, &mut camera);
        let shadow = ShadowMap::new(device, MAP_SIZE, STRENGTH)?;
        let shadow_debug = ShadowDebug::new(device, &shadow.map);
        Ok(Self {
            camera,
            control,
            splats: None,
            petal: None,
            depth: depth_view(device, [1, 1]),
            shadow,
            shadow_debug,
            show_shadow_camera: true,
            show_petal: true,
            petal_size: 0.02,
            petal_count: MIN_PETALS,
            petal_motion: MotionSettings::default(),
            flowers: None,
            show_flower_markers: false,
            splat_hide_distance: 3.5,
            sort_positions: Vec::new(),
            sort_gate: SortGate::default(),
            pending_sort: None,
            viewport: [1, 1],
            sort_count: 0,
            source_count: 0,
        })
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) -> BelfastResult<()> {
        let aspect = width as f32 / height as f32;
        self.camera.set_aspect(aspect)?;
        self.camera.set_fovy_radians(blender_fovy(aspect))?;
        self.viewport = [width, height];
        self.depth = depth_view(device, self.viewport);
        Ok(())
    }

    pub fn load_ply(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let mut cloud = GaussianCloud::parse(bytes)?;
        let renderer =
            SplatRenderer::new_shadowed(device, &mut cloud, Some(DEPTH_FORMAT), &self.shadow)?;
        let count = renderer.count();
        self.source_count = cloud.source_count;
        self.sort_positions = cloud.sort_positions;
        self.splats = Some(renderer);
        self.sort_gate = SortGate::default();
        self.pending_sort = None;
        self.sort_count = 0;
        Ok(count)
    }

    pub fn take_sort_positions(&mut self) -> Vec<f32> {
        let positions = std::mem::take(&mut self.sort_positions);
        positions.into_iter().flatten().collect()
    }

    pub fn load_flower_emitters(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let flowers = FlowerMarkers::new(device, bytes)?;
        let count = flowers.emitters.len() as u32;
        if let Some(petal) = &mut self.petal {
            petal.reset_instances(device, &flowers.emitters, self.petal_count)?;
        }
        self.flowers = Some(flowers);
        Ok(count)
    }

    pub fn load_petal(&mut self, device: &Device, bytes: &[u8]) -> Result<(), String> {
        let flowers = self
            .flowers
            .as_ref()
            .ok_or("Load flower positions before petals")?;
        self.petal = Some(Petal::new(
            device,
            bytes,
            &flowers.emitters,
            self.petal_count,
            &self.shadow,
        )?);
        Ok(())
    }

    pub fn set_petal_count(&mut self, device: &Device, count: u32) -> Result<(), String> {
        if !(MIN_PETALS..=MAX_PETALS).contains(&count) {
            return Err("Petal count must be between 10,000 and 100,000".into());
        }
        if let Some(petal) = &mut self.petal {
            let flowers = self
                .flowers
                .as_ref()
                .ok_or("Petals require flower positions")?;
            petal.ensure_instances(device, &flowers.emitters, count)?;
        }
        self.petal_count = count;
        Ok(())
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

    pub fn set_shadow_strength(&mut self, device: &Device, strength: f32) -> Result<(), String> {
        self.shadow.set_strength(device, strength)
    }

    pub fn set_petal_motion(
        &mut self,
        enabled: bool,
        updraft: f32,
        strength: f32,
    ) -> Result<(), String> {
        if !updraft.is_finite()
            || !(0.0..=0.6).contains(&updraft)
            || !strength.is_finite()
            || !(0.0..=2.0).contains(&strength)
        {
            return Err("Updraft must be 0–0.6 and vortex strength 0–2".into());
        }
        self.petal_motion = MotionSettings {
            enabled,
            updraft,
            strength,
        };
        Ok(())
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32) -> BelfastResult<()> {
        self.control.update(delta_seconds, &mut self.camera);
        if let Some(splats) = &self.splats {
            splats.update_camera(device, &self.sort_camera(), self.splat_hide_distance);
        }
        if let Some(flowers) = &self.flowers {
            flowers.update_camera(device, &self.camera.view_projection_matrix());
        }
        if let Some(petal) = &mut self.petal {
            self.shadow
                .fit(device, petal.shadow_bounds(self.petal_size))?;
            petal.update(
                device,
                &self.camera.view_projection_matrix(),
                self.petal_size,
            );
            petal.prepare_motion(
                device,
                delta_seconds,
                self.petal_count,
                self.petal_motion,
                self.shadow.view_projection(),
            );
        }
        self.shadow
            .update_viewer(device, &self.camera.view_projection_matrix());
        Ok(())
    }

    pub fn render(&mut self, device: &Device, view: &wgpu::TextureView) {
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("CourtyardEncoder"),
            });
        if let Some(petal) = &mut self.petal {
            petal.simulate(&mut encoder);
        }
        {
            // Clear even when petals are hidden so the scene cannot retain stale shadows.
            let mut pass = self
                .shadow
                .map
                .begin_depth_pass(&mut encoder, wgpu::LoadOp::Clear(1.0));
            if self.show_petal {
                if let Some(petal) = &self.petal {
                    petal.draw_shadow(&mut pass, self.petal_count);
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CourtyardScene"),
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
                    view: &self.depth,
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
            // Opaque petals establish depth; nearer sorted splats blend over them.
            if self.show_petal {
                if let Some(petal) = &self.petal {
                    petal.draw(&mut pass, self.petal_count);
                }
            }
            if let Some(splats) = &self.splats {
                splats.draw(&mut pass);
            }
        }
        let flowers = self.flowers.as_ref().filter(|_| self.show_flower_markers);
        if (self.show_shadow_camera && self.petal.is_some()) || flowers.is_some() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CourtyardDebugOverlay"),
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
            // Draw the exact shadow frustum in front of the scene, without casting shadows.
            if self.show_shadow_camera {
                self.shadow.draw_camera(&mut pass);
            }
            if let Some(flowers) = flowers {
                flowers.draw(&mut pass);
            }
        }
        self.shadow_debug.render(&mut encoder, view, self.viewport);
        device.queue().submit([encoder.finish()]);
    }
}
