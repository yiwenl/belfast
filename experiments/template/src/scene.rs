use belfast::{
    AxisHelper, AxisHelperOptions, BelfastResult, BindGroup, Buffer, BufferUsage, Device,
    OrbitalControl, OrbitalControlOptions, PerspectiveCamera,
};

use crate::input::SkeletonInput;

/// The experiment lives here. Add particle buffers, compute and drawing to this type.
pub struct Scene {
    pub camera: PerspectiveCamera,
    pub control: OrbitalControl,
    pub skeleton: SkeletonInput,
    axes: AxisHelper,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
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
        let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.01, 100.0);
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            radius: 7.0,
            min_radius: 0.25,
            max_radius: 50.0,
            ..Default::default()
        })?;
        control.snap_yaw(40.0_f32.to_radians());
        control.snap_pitch(25.0_f32.to_radians());
        control.update(0.0, &mut camera);
        Ok(Self {
            camera,
            control,
            skeleton: SkeletonInput::default(),
            axes,
            camera_buffer,
            camera_bind_group,
        })
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        self.control.update(delta_seconds, &mut self.camera);
        self.camera_buffer
            .write(device, &self.camera.view_projection_matrix(), 0);

        // Future interaction hook: this is None after tracking is lost or becomes stale.
        // Consume the points here to update attractors or a GPU skeleton buffer.
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&self, device: &Device, view: &wgpu::TextureView) {
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ExperimentEncoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ExperimentAxes"),
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
            self.axes.draw(&mut pass, &self.camera_bind_group);
        }
        device.queue().submit([encoder.finish()]);
    }
}
