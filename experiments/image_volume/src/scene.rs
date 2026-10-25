use wgpu::util::DeviceExt;

use belfast::{
    AxisHelper, AxisHelperOptions, BelfastResult, BindGroup, Buffer, BufferUsage, Device,
    OrbitalControl, OrbitalControlOptions, PerspectiveCamera,
};

use crate::input::SkeletonInput;

const FRAMES: u32 = 190;
const SIZE: u32 = 1024;
const UNIFORM_FLOATS: usize = 36;

/// The experiment lives here. Add particle buffers, compute and drawing to this type.
pub struct Scene {
    pub camera: PerspectiveCamera,
    pub control: OrbitalControl,
    pub plane_control: OrbitalControl,
    pub skeleton: SkeletonInput,
    plane_camera: PerspectiveCamera,
    face_on: bool,
    show_slices: bool,
    axes: AxisHelper,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    volume: wgpu::Texture,
    plane_pipeline: wgpu::RenderPipeline,
}

impl Scene {
    pub fn new(device: &Device) -> BelfastResult<Self> {
        let camera_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ImageVolumeLayout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ImageVolumePipelineLayout"),
                    bind_group_layouts: &[Some(&camera_layout)],
                    immediate_size: 0,
                });
        let axes = AxisHelper::new(
            device,
            AxisHelperOptions {
                length: 1.0,
                ..AxisHelperOptions::new(device.format(), &pipeline_layout)
            },
        )?;
        let volume = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("ImageVolume"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: FRAMES,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let volume_view = volume.create_view(&wgpu::TextureViewDescriptor {
            label: Some("ImageVolumeView"),
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
        let sampler = device.gpu().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ImageVolumeSampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let camera_buffer = Buffer::create(
            device,
            (UNIFORM_FLOATS * 4) as u64,
            BufferUsage::uniform(),
            "ImageVolumeUniform",
        );
        let camera_bind_group = BindGroup::create(
            device,
            &camera_layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&volume_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            "ImageVolumeBindGroup",
        );
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ImageVolumeSlice"),
                source: wgpu::ShaderSource::Wgsl(include_str!("slice.wgsl").into()),
            });
        let plane_pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ImageVolumePlane"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: device.format(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.01, 100.0);
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            radius: 2.5,
            min_radius: 0.25,
            max_radius: 50.0,
            ..Default::default()
        })?;
        control.snap_yaw(0.0);
        control.snap_pitch(0.0);
        control.update(0.0, &mut camera);
        let mut plane_camera =
            PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.01, 100.0);
        let mut plane_control = OrbitalControl::new(OrbitalControlOptions {
            radius: 1.0,
            min_radius: 1.0,
            max_radius: 1.0,
            ..Default::default()
        })?;
        plane_control.update(0.0, &mut plane_camera);
        Ok(Self {
            camera,
            control,
            plane_control,
            skeleton: SkeletonInput::default(),
            plane_camera,
            face_on: true,
            show_slices: false,
            axes,
            camera_buffer,
            camera_bind_group,
            volume,
            plane_pipeline,
        })
    }

    pub fn set_face_on(&mut self, face_on: bool) {
        self.face_on = face_on;
    }

    /// Draw every photograph as its own slab. The camera steps aside so the slabs
    /// do not stack on the same pixels, and the cutting plane stays in the cube.
    pub fn set_show_slices(&mut self, show: bool) {
        self.show_slices = show;
        if show {
            self.face_on = false;
            self.control.snap_yaw(0.65);
            self.control.snap_pitch(0.5);
        } else {
            self.control.snap_yaw(0.0);
            self.control.snap_pitch(0.0);
        }
    }

    pub fn upload_frame(&self, device: &Device, index: u32, pixels: &[u8]) -> Result<(), String> {
        validate_frame(index, pixels.len())?;
        // write_texture stays queued until a later submit, and an empty submit does
        // not flush it. Dawn then packs all 190 frames into one 760 MB staging buffer.
        let staging = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ImageVolumeFrame"),
                contents: pixels,
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ImageVolumeUpload"),
            });
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.volume,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: index,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        device.queue().submit([encoder.finish()]);
        Ok(())
    }

    pub fn update(&mut self, device: &Device, delta_seconds: f32, now_ms: f64) {
        self.control.update(delta_seconds, &mut self.camera);
        self.plane_control
            .update(delta_seconds, &mut self.plane_camera);
        let mut uniform = [0.0; UNIFORM_FLOATS];
        uniform[..16].copy_from_slice(&self.camera.view_projection_matrix());
        uniform[16..32].copy_from_slice(&plane_rotation(
            self.plane_control.yaw(),
            self.plane_control.pitch(),
        ));
        uniform[32] = if self.face_on { 1.0 } else { 0.0 };
        uniform[33] = if self.show_slices { FRAMES as f32 } else { 0.0 };
        uniform[34] = FRAMES as f32;
        self.camera_buffer.write(device, &uniform, 0);

        // Future interaction hook: this is None after tracking is lost or becomes stale.
        let _skeleton = self.skeleton.latest(now_ms);
    }

    pub fn render(&self, device: &Device, view: &wgpu::TextureView) {
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ImageVolumeEncoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ImageVolume"),
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
            pass.set_pipeline(&self.plane_pipeline);
            self.camera_bind_group.bind(&mut pass, 0);
            let slices = if self.show_slices { FRAMES } else { 0 };
            pass.draw(0..slices * 6 + 6, 0..1);
        }
        device.queue().submit([encoder.finish()]);
    }
}

/// Yaw around Y, then pitch around world X. The orbit camera's view matrix rolls
/// around Z as soon as yaw is nonzero, so vertical drag was twisting the plane.
fn plane_rotation(yaw: f32, pitch: f32) -> [f32; 16] {
    (glam::Mat4::from_rotation_x(pitch) * glam::Mat4::from_rotation_y(-yaw)).to_cols_array()
}

/// Same mapping as `slice.wgsl`. z = 0 lands between frames 94 and 95 because 190 is even.
#[cfg(test)]
fn volume_uvw(point: [f32; 3]) -> [f32; 3] {
    [
        point[0] * 0.5 + 0.5,
        0.5 - point[1] * 0.5,
        point[2] * 0.5 + 0.5,
    ]
}

/// World z of one photograph, matching the slice branch in `slice.wgsl`.
#[cfg(test)]
fn slice_z(index: u32) -> f32 {
    ((index as f32 + 0.5) / FRAMES as f32) * 2.0 - 1.0
}

fn validate_frame(index: u32, len: usize) -> Result<(), String> {
    if index >= FRAMES {
        return Err(format!("Frame {index} is outside 0..{FRAMES}"));
    }
    let expected = (SIZE * SIZE * 4) as usize;
    if len != expected {
        return Err(format!(
            "Frame {index} has {len} bytes; expected {expected}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{plane_rotation, slice_z, validate_frame, volume_uvw, FRAMES, SIZE};

    #[test]
    fn slice_shader_parses() {
        naga::front::wgsl::parse_str(include_str!("slice.wgsl")).unwrap();
    }

    use belfast::{OrbitalControl, OrbitalControlOptions, PerspectiveCamera};

    #[test]
    fn frame_upload_rejects_a_bad_index_or_size() {
        assert!(validate_frame(0, (SIZE * SIZE * 4) as usize).is_ok());
        assert!(validate_frame(FRAMES, (SIZE * SIZE * 4) as usize).is_err());
        assert!(validate_frame(0, 4).is_err());
    }

    #[test]
    fn each_slice_lands_on_its_own_frame() {
        let first = volume_uvw([0.0, 0.0, slice_z(0)]);
        let last = volume_uvw([0.0, 0.0, slice_z(FRAMES - 1)]);
        assert!((first[2] - 0.5 / FRAMES as f32).abs() < 1e-5);
        assert!((last[2] - (FRAMES as f32 - 0.5) / FRAMES as f32).abs() < 1e-5);
        assert!(slice_z(0) < slice_z(FRAMES - 1));
    }

    #[test]
    fn an_unrotated_plane_samples_the_middle_of_the_volume() {
        let center = volume_uvw([0.0, 0.0, 0.0]);
        assert_eq!(center, [0.5, 0.5, 0.5]);
        let corner = volume_uvw([0.5, 0.5, 0.0]);
        assert!((corner[0] - 0.75).abs() < 1e-6);
        assert!((corner[1] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn yaw_quarter_turn_puts_the_plane_edge_on_to_the_front_camera() {
        let mut hidden = PerspectiveCamera::new(1.0, 1.0, 0.1, 10.0);
        let mut control = OrbitalControl::new(OrbitalControlOptions {
            radius: 1.0,
            min_radius: 1.0,
            max_radius: 1.0,
            ..Default::default()
        })
        .unwrap();
        control.update(0.0, &mut hidden);
        let facing = glam::Mat4::from_cols_array(&plane_rotation(control.yaw(), control.pitch()));
        let facing_normal = facing.transform_vector3(glam::Vec3::Z);
        assert!(facing_normal.x.abs() + facing_normal.y.abs() < 1e-4);
        assert!((facing_normal.z.abs() - 1.0).abs() < 1e-4);

        control.snap_pitch(0.6);
        control.update(0.0, &mut hidden);
        let pitched = glam::Mat4::from_cols_array(&plane_rotation(control.yaw(), control.pitch()));
        let up = pitched.transform_point3(glam::Vec3::Y * 0.5);
        assert!(
            up.x.abs() < 1e-4,
            "vertical drag should hinge on X, got {up}"
        );
        assert!(up.z.abs() > 0.2);

        control.snap_yaw(0.8);
        control.update(0.0, &mut hidden);
        let pitched_after_yaw =
            glam::Mat4::from_cols_array(&plane_rotation(control.yaw(), control.pitch()));
        let up = pitched_after_yaw.transform_point3(glam::Vec3::Y);
        assert!(
            up.x.abs() < 1e-4,
            "pitch stays on world X after yaw, got {up}"
        );

        control.snap_pitch(0.0);
        control.snap_yaw(std::f32::consts::FRAC_PI_2);
        control.update(0.0, &mut hidden);
        let model = glam::Mat4::from_cols_array(&plane_rotation(control.yaw(), control.pitch()));
        assert_eq!(model.w_axis, glam::Vec4::W);
        let normal = model.transform_vector3(glam::Vec3::Z);
        assert!(
            normal.z.abs() < 1e-4,
            "normal {normal} should be perpendicular to the front view"
        );
    }
}
