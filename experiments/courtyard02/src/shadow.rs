use belfast::{
    BelfastResult, BindGroup, Buffer, BufferUsage, CameraHelper, CameraHelperOptions, Device,
    Texture, TextureOptions,
};
use glam::{Mat4, Vec3};

pub const MAP_SIZE: u32 = 2048;
pub const STRENGTH: f32 = 0.65;
pub const SAMPLING_SHADER: &str = include_str!("shadow-sampling.wgsl");
const LIGHT_POSITION: Vec3 = Vec3::new(0.0, 8.0, 3.0);

pub struct ShadowMap {
    pub map: Texture,
    uniform: Buffer,
    pub sample_layout: wgpu::BindGroupLayout,
    pub sample_binding: wgpu::BindGroup,
    pub depth_layout: wgpu::BindGroupLayout,
    pub depth_binding: wgpu::BindGroup,
    strength: f32,
    bounds: Option<(Vec3, Vec3)>,
    view_projection: [f32; 16],
    camera_helper: CameraHelper,
    viewer_camera: Buffer,
    viewer_binding: BindGroup,
}

impl ShadowMap {
    pub fn new(device: &Device, size: u32, strength: f32) -> BelfastResult<Self> {
        let map = Texture::create_2d(
            device,
            size,
            size,
            TextureOptions::shadow_map("CourtyardShadow"),
        )?;
        let uniform = Buffer::create(device, 96, BufferUsage::uniform(), "CourtyardLight");
        let uniform_entry = |visibility| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let sample_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CourtyardShadowSamplingLayout"),
                    entries: &[
                        uniform_entry(wgpu::ShaderStages::VERTEX_FRAGMENT),
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
                    ],
                });
        let sample_binding = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CourtyardShadowSamplingBinding"),
            layout: &sample_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(map.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(map.sampler()),
                },
            ],
        });
        // The depth pass must not sample the texture it is writing.
        let depth_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CourtyardShadowDepthLayout"),
                    entries: &[uniform_entry(wgpu::ShaderStages::VERTEX)],
                });
        let depth_binding = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CourtyardShadowDepthBinding"),
            layout: &depth_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.gpu().as_entire_binding(),
            }],
        });
        let helper_layout = device
            .gpu()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ShadowCameraHelperLayout"),
                bind_group_layouts: &[Some(&depth_layout)],
                immediate_size: 0,
            });
        let camera_helper = CameraHelper::new(
            device,
            &Mat4::IDENTITY.to_cols_array(),
            CameraHelperOptions {
                label: "CourtyardShadowCamera",
                color: [1.0, 0.8, 0.1],
                ..CameraHelperOptions::new(device.format(), &helper_layout)
            },
        )?;
        let viewer_camera =
            Buffer::create(device, 64, BufferUsage::uniform(), "ShadowCameraViewer");
        let viewer_binding = BindGroup::create(
            device,
            &depth_layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewer_camera.gpu().as_entire_binding(),
            }],
            "ShadowCameraViewerBinding",
        );
        let shadow = Self {
            map,
            uniform,
            sample_layout,
            sample_binding,
            depth_layout,
            depth_binding,
            strength,
            bounds: None,
            view_projection: Mat4::IDENTITY.to_cols_array(),
            camera_helper,
            viewer_camera,
            viewer_binding,
        };
        shadow.update_matrix(device, Mat4::IDENTITY);
        Ok(shadow)
    }

    pub fn fit(&mut self, device: &Device, bounds: (Vec3, Vec3)) -> BelfastResult<()> {
        if self.bounds == Some(bounds) {
            return Ok(());
        }
        let view = Mat4::look_at_rh(LIGHT_POSITION, Vec3::ZERO, Vec3::Y);
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for x in [bounds.0.x, bounds.1.x] {
            for y in [bounds.0.y, bounds.1.y] {
                for z in [bounds.0.z, bounds.1.z] {
                    let p = view.transform_point3(Vec3::new(x, y, z));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
        // Expand around the fitted center to leave room for drifting petals.
        let margin = 0.02;
        let center = (min + max) * 0.5;
        let half_width = ((max.x - min.x) * 0.5 + margin) * 3.0;
        let half_height = ((max.y - min.y) * 0.5 + margin) * 6.0;
        let matrix = Mat4::orthographic_rh(
            center.x - half_width,
            center.x + half_width,
            center.y - half_height,
            center.y + half_height,
            // From above the flowers to below the floor, relative to the light at Y = 8.
            5.0,
            9.5,
        ) * view;
        self.camera_helper.set_matrix(&matrix.to_cols_array())?;
        self.view_projection = matrix.to_cols_array();
        self.update_matrix(device, matrix);
        self.bounds = Some(bounds);
        Ok(())
    }

    pub fn update_viewer(&self, device: &Device, view_projection: &[f32; 16]) {
        self.viewer_camera.write(device, view_projection, 0);
    }

    pub fn view_projection(&self) -> [f32; 16] {
        self.view_projection
    }

    pub fn set_strength(&mut self, device: &Device, strength: f32) -> Result<(), String> {
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
            return Err("Shadow strength must be between 0 and 1".into());
        }
        self.strength = strength;
        self.uniform.write(device, &[strength], 64);
        Ok(())
    }

    pub fn draw_camera<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.bounds.is_some() {
            self.camera_helper.draw(pass, &self.viewer_binding);
        }
    }

    fn update_matrix(&self, device: &Device, matrix: Mat4) {
        let mut values = matrix.to_cols_array().to_vec();
        // Strength, map size, comparison depth bias, world-space normal bias.
        values.extend([self.strength, self.map.width() as f32, 0.00002, 0.0005]);
        values.extend(LIGHT_POSITION.normalize().extend(0.0).to_array());
        self.uniform.write(device, &values, 0);
    }
}
