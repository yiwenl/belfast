use belfast::{BelfastResult, Device, OrbitalControl, OrbitalControlOptions, PerspectiveCamera};
use glam::{Mat4, Vec3};

use crate::{
    fireflies::{depth_view, merged_depth, FireflySystem},
    fog::Fog,
    forest_color::ForestColor,
    gpu_timing::{render_timestamps, GpuTiming},
    input::SkeletonInput,
    light_volume::LightVolume,
    ply::GaussianCloud,
    sorting::{SortCamera, SortGate},
    splat::SplatRenderer,
};

/// Rust owns the forest renderer and camera; a worker supplies sorted indices.
pub struct Scene {
    pub camera: PerspectiveCamera,
    pub control: OrbitalControl,
    pub skeleton: SkeletonInput,
    pub viewport: [u32; 2],
    splats: Option<SplatRenderer>,
    sort_positions: Vec<[f32; 4]>,
    sort_gate: SortGate,
    pending_sort: Option<SortCamera>,
    fireflies: FireflySystem,
    forest_color: ForestColor,
    light_volume: LightVolume,
    fog: Fog,
    depth: wgpu::TextureView,
    depth_size: [u32; 2],
    forest_camera: Option<SortCamera>,
    forest_dirty: bool,
    camera_frozen: bool,
    benchmark_restore: Option<PerspectiveCamera>,
    benchmark_path_started: bool,
    gpu_timing: Option<GpuTiming>,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let mut camera = PerspectiveCamera::new(50.0_f32.to_radians(), 1.0, 0.02, 200.0);
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            center: [0.0, 0.0, 0.0],
            radius: 5.0,
            min_radius: 0.05,
            max_radius: 100.0,
            rotate_sensitivity: 0.003,
            ..Default::default()
        })?;
        control.update(0.0, &mut camera);
        let depth = depth_view(device, [1, 1]);
        let mut fireflies = FireflySystem::new(device);
        fireflies.set_occlusion(device, &depth);
        let light_volume = LightVolume::new(device, &fireflies);
        let fog = Fog::new(device, &depth);
        let forest_color = ForestColor::new(device, &light_volume, &fog, &depth);
        Ok(Self {
            camera,
            control,
            skeleton: SkeletonInput::default(),
            viewport: [1, 1],
            splats: None,
            sort_positions: Vec::new(),
            sort_gate: SortGate::default(),
            pending_sort: None,
            fireflies,
            forest_color,
            light_volume,
            fog,
            depth,
            depth_size: [1, 1],
            forest_camera: None,
            forest_dirty: true,
            camera_frozen: false,
            benchmark_restore: None,
            benchmark_path_started: false,
            gpu_timing: None,
        })
    }

    pub fn load_ply(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let mut cloud = GaussianCloud::parse(bytes)?;
        let renderer = SplatRenderer::new(device, &mut cloud)?;
        let count = renderer.count();
        self.sort_positions = cloud.sort_positions;
        self.splats = Some(renderer);
        self.sort_gate = SortGate::default();
        self.pending_sort = None;
        self.forest_dirty = true;
        Ok(count)
    }

    pub fn take_sort_positions(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.sort_positions)
            .into_iter()
            .flatten()
            .collect()
    }

    fn sort_camera(&self) -> SortCamera {
        // All passes and sorting share scene-local coordinates, including fog and light volumes.
        let scene_transform = Mat4::from_translation(Vec3::new(0., -0.5, 0.));
        SortCamera::new(
            Mat4::from_cols_array(&self.camera.view_matrix()) * scene_transform,
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

    pub fn apply_sort(&mut self, device: &Device, indices: &[u32]) -> Result<(), String> {
        let camera = self.pending_sort.ok_or("No Gaussian sort is pending")?;
        self.splats
            .as_mut()
            .ok_or("No Gaussian scene loaded")?
            .set_indices(device, indices)?;
        self.sort_gate.accept(camera);
        if self.camera_frozen && !self.benchmark_path_started {
            let current = self.sort_camera();
            if camera.view != current.view || camera.projection != current.projection {
                self.sort_gate = SortGate::default();
            }
        }
        self.pending_sort = None;
        self.forest_dirty = true;
        Ok(())
    }

    pub fn visible_count(&self) -> u32 {
        self.splats.as_ref().map_or(0, SplatRenderer::visible)
    }

    pub fn set_camera_fov(&mut self, degrees: f32) -> Result<f32, String> {
        if !degrees.is_finite() {
            return Err("Camera FOV must be finite".into());
        }
        let degrees = degrees.clamp(20.0, 100.0);
        self.camera
            .set_fovy_radians(degrees.to_radians())
            .map_err(|error| error.to_string())?;
        Ok(degrees)
    }

    pub fn set_firefly_rates(&mut self, movement: f32, blinking: f32) -> Result<(), String> {
        self.fireflies.set_rates(movement, blinking)
    }

    pub fn set_firefly_count(&mut self, count: u32) -> u32 {
        self.light_volume.invalidate();
        self.fireflies.set_count(count)
    }

    pub fn set_firefly_flash_duration(&mut self, seconds: f32) -> Result<(), String> {
        self.fireflies.set_flash_duration(seconds)
    }

    pub fn set_firefly_randomness(&mut self, movement: f32, blinking: f32) -> Result<(), String> {
        self.fireflies.set_randomness(movement, blinking)
    }

    pub fn set_firefly_size(&mut self, scale: f32) -> Result<(), String> {
        self.fireflies.set_size(scale)
    }

    pub fn set_view_culling(&mut self, enabled: bool) {
        self.fireflies.set_view_culling(enabled);
    }

    pub fn set_forest_grade(
        &self,
        device: &Device,
        strength: f32,
        brightness: f32,
        colors: &[f32],
    ) -> Result<(), String> {
        self.forest_color
            .set_grade(device, strength, brightness, colors)
    }

    pub fn set_firefly_appearance(
        &mut self,
        device: &Device,
        min_peak: f32,
        max_peak: f32,
        warm_share: f32,
        colors: &[f32],
    ) -> Result<(), String> {
        self.fireflies
            .set_appearance(device, min_peak, max_peak, warm_share, colors)?;
        self.light_volume.invalidate();
        Ok(())
    }

    pub fn set_light_volume(
        &mut self,
        device: &Device,
        enabled: bool,
        strength: f32,
        radius: f32,
        hz: f32,
    ) -> Result<(), String> {
        self.light_volume
            .configure(device, enabled, strength, radius, hz)
    }

    pub fn set_fog(
        &mut self,
        enabled: bool,
        density: f32,
        speed: f32,
        light: f32,
        noise_frequency: f32,
    ) -> Result<(), String> {
        self.fog
            .configure(enabled, density, speed, light, noise_frequency)
    }

    pub fn begin_benchmark(&mut self, device: &Device, seed: u32, moving: bool) {
        // Finish orbital damping before capturing the view, so a completed run
        // does not resume old camera motion and invalidate the next A/B run.
        self.control.update(10., &mut self.camera);
        self.control.set_enabled(false);
        self.camera_frozen = true;
        self.benchmark_path_started = false;
        if moving {
            self.benchmark_restore = Some(self.camera.clone());
            self.set_benchmark_camera(0.0);
        }
        self.gpu_timing = None;
        // Force a sort of this exact view even below the ordinary motion threshold.
        self.sort_gate = SortGate::default();
        self.fireflies.reset(device, seed);
        self.fog.reset();
        self.light_volume.invalidate();
    }

    // forest-loop-v1: 20 seconds / 1200 fixed steps, never driven by wall time.
    fn set_benchmark_camera(&mut self, seconds: f32) {
        let phase = seconds * std::f32::consts::TAU / 20.0;
        let eye = glam::Vec3::new(
            0.35 * phase.sin(),
            0.06 * (2.0 * phase).sin(),
            -0.4 * (1.0 - phase.cos()),
        );
        let yaw = 0.3 * phase.sin();
        let pitch = 0.08 * (2.0 * phase).sin();
        let forward = glam::Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        self.camera
            .look_at(eye.to_array(), (eye + forward).to_array());
    }

    pub fn reset_benchmark(&mut self, device: &Device, seed: u32, frames: u32) {
        self.benchmark_path_started = true;
        self.fireflies.reset(device, seed);
        self.fog.reset();
        self.light_volume.invalidate();
        self.gpu_timing = device
            .gpu()
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
            .then(|| GpuTiming::new(device, frames));
    }

    pub fn end_benchmark(&mut self) {
        if let Some(camera) = self.benchmark_restore.take() {
            self.camera = camera;
        }
        self.benchmark_path_started = false;
        self.camera_frozen = false;
        self.control.set_enabled(true);
        self.gpu_timing = None;
    }

    pub fn benchmark_ready(&self) -> bool {
        self.splats.is_some()
            && self.pending_sort.is_none()
            && !self.sort_gate.needs_sort(&self.sort_camera())
            && !self.forest_dirty
    }

    pub fn camera_snapshot(&self) -> Vec<f32> {
        self.sort_camera().encode()
    }

    pub fn finish_gpu_timing(&mut self, device: &Device) {
        if let Some(timing) = &mut self.gpu_timing {
            timing.finish(device);
        }
    }

    pub fn take_gpu_timings(&mut self) -> Result<Vec<f64>, String> {
        match &mut self.gpu_timing {
            Some(timing) => Ok(timing.take()?.unwrap_or_default()),
            None => Ok(Vec::new()),
        }
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        if !self.camera_frozen {
            self.control.update(delta_seconds, &mut self.camera);
        }
        if self.benchmark_restore.is_some() && self.benchmark_path_started {
            self.set_benchmark_camera((now_ms * 0.001) as f32);
        }
        let camera = self.sort_camera();
        if self.depth_size != self.viewport {
            self.depth = depth_view(device, self.viewport);
            self.fireflies.set_occlusion(device, &self.depth);
            self.fog.resize(device, self.viewport, &self.depth);
            self.forest_color.resize(
                device,
                self.viewport,
                &self.light_volume,
                &self.fog,
                &self.depth,
            );
            self.depth_size = self.viewport;
            self.forest_dirty = true;
        }
        if self
            .forest_camera
            .is_none_or(|old| old.view != camera.view || old.projection != camera.projection)
        {
            self.forest_dirty = true;
        }
        self.fireflies.update(device, delta_seconds, &camera);
        self.forest_color.update_camera(device, &camera);
        self.fog.update(device, &camera, delta_seconds);
        self.light_volume.advance(delta_seconds);
        if let Some(splats) = &self.splats {
            splats.update_camera(device, &camera);
        }
        // Retain the template's optional skeleton input for future interaction.
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&mut self, device: &Device, view: &wgpu::TextureView) {
        let queries = self
            .gpu_timing
            .as_ref()
            .filter(|timing| timing.recording())
            .map(|timing| &timing.queries);
        let merged = merged_depth(device);
        let had_depth = self.forest_dirty && !merged;
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ForestEncoder"),
            });
        if self.splats.is_some() {
            self.fireflies.simulate_profiled(&mut encoder, queries);
        }
        if had_depth {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ForestOcclusion"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: render_timestamps(queries, 6),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(splats) = &self.splats {
                splats.draw_depth(&mut pass);
            }
        }
        let light_updated = self.splats.is_some()
            && self
                .light_volume
                .render(&mut encoder, &self.fireflies, queries);
        if self.forest_dirty {
            let attachments = [
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.forest_color.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                merged.then_some(wgpu::RenderPassColorAttachment {
                    view: &self.depth,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ForestGaussians"),
                color_attachments: &attachments[..if merged { 2 } else { 1 }],
                depth_stencil_attachment: None,
                // Volume updates start this span earlier; finish after copying.
                timestamp_writes: queries.filter(|_| !light_updated).map(|query_set| {
                    wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(8),
                        end_of_pass_write_index: None,
                    }
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(splats) = &self.splats {
                splats.draw(&mut pass);
            }
        }
        // Fog uses the current depth, while its animation leaves the forest cache valid.
        let fog_rendered = self.fog.render(
            &mut encoder,
            queries.filter(|_| !self.forest_dirty && !light_updated),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ForestColorCopy"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: queries.map(|query_set| wgpu::RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: (!self.forest_dirty
                        && !light_updated
                        && !fog_rendered)
                        .then_some(8),
                    end_of_pass_write_index: Some(9),
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.forest_color.draw(&mut pass);
        }
        self.forest_camera = Some(self.sort_camera());
        self.forest_dirty = false;
        if self.splats.is_some() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FireflyGlow"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: (!merged).then_some(
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    },
                ),
                timestamp_writes: render_timestamps(queries, 10),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.fireflies.draw(&mut pass);
        }
        if let Some(timing) = &mut self.gpu_timing {
            timing.record(&mut encoder, had_depth);
        }
        device.queue().submit([encoder.finish()]);
    }
}
