use crate::{
    markers::FlowerMarkers,
    ply::GaussianCloud,
    sorting::{SortCamera, SortGate},
    splat::SplatRenderer,
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
    pub flowers: Option<FlowerMarkers>,
    pub show_flower_markers: bool,
    sort_positions: Vec<[f32; 4]>,
    sort_gate: SortGate,
    pending_sort: Option<SortCamera>,
    pub viewport: [u32; 2],
    pub sort_count: u32,
    pub source_count: u32,
}

impl Scene {
    pub fn new(_device: &Device) -> BelfastResult<Self> {
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
        Ok(Self {
            camera,
            control,
            splats: None,
            flowers: None,
            show_flower_markers: true,
            sort_positions: Vec::new(),
            sort_gate: SortGate::default(),
            pending_sort: None,
            viewport: [1, 1],
            sort_count: 0,
            source_count: 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> BelfastResult<()> {
        let aspect = width as f32 / height as f32;
        self.camera.set_aspect(aspect)?;
        self.camera.set_fovy_radians(blender_fovy(aspect))?;
        self.viewport = [width, height];
        Ok(())
    }

    pub fn load_ply(&mut self, device: &Device, bytes: &[u8]) -> Result<u32, String> {
        let mut cloud = GaussianCloud::parse(bytes)?;
        let renderer = SplatRenderer::new(device, &mut cloud)?;
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
        self.flowers = Some(flowers);
        Ok(count)
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

    pub fn update(&mut self, device: &Device, delta_seconds: f32) {
        self.control.update(delta_seconds, &mut self.camera);
        if let Some(splats) = &self.splats {
            splats.update_camera(device, &self.sort_camera());
        }
        if let Some(flowers) = &self.flowers {
            flowers.update_camera(device, &self.camera.view_projection_matrix());
        }
    }

    pub fn render(&mut self, device: &Device, view: &wgpu::TextureView) {
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("CourtyardEncoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CourtyardSplats"),
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
            if let Some(splats) = &self.splats {
                splats.draw(&mut pass);
            }
            if self.show_flower_markers {
                if let Some(flowers) = &self.flowers {
                    flowers.draw(&mut pass);
                }
            }
        }
        device.queue().submit([encoder.finish()]);
    }
}
