use belfast::{BelfastResult, Buffer, BufferUsage, Device, Texture, TextureOptions};
use glam::{Mat4, Vec2, Vec3};

pub const MAP_SIZE: u32 = 2048;
pub const STRENGTH: f32 = 0.85;
pub const DEPTH_SHADER: &str = include_str!("shadow-depth.wgsl");
pub const SAMPLING_SHADER: &str = include_str!("shadow-sampling.wgsl");

/// A perspective shadow camera shares the point light's exact origin.
/// Fit every AABB corner, including flight and mesh-size padding, once at load.
pub fn fit_shadow_camera(eye: Vec3, min: Vec3, max: Vec3) -> Result<Mat4, String> {
    if !eye.is_finite()
        || !min.is_finite()
        || !max.is_finite()
        || min.cmpge(max).any()
        || eye.y <= max.y
    {
        return Err("Shadow light must be above the finite flight bounds".into());
    }
    let center = (min + max) * 0.5;
    let direction = (center - eye).normalize();
    let up = if direction.dot(Vec3::Y).abs() > 0.95 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let view = Mat4::look_at_rh(eye, center, up);
    let mut near = f32::INFINITY;
    let mut far = 0f32;
    let mut min_slope = Vec2::splat(f32::INFINITY);
    let mut max_slope = Vec2::splat(f32::NEG_INFINITY);
    for x in [min.x, max.x] {
        for y in [min.y, max.y] {
            for z in [min.z, max.z] {
                let p = view.transform_point3(Vec3::new(x, y, z));
                let depth = -p.z;
                if depth <= 0. {
                    return Err("Flight bounds cross the shadow light plane".into());
                }
                near = near.min(depth);
                far = far.max(depth);
                let slope = Vec2::new(p.x, p.y) / depth;
                min_slope = min_slope.min(slope);
                max_slope = max_slope.max(slope);
            }
        }
    }
    // Fit each side independently rather than wasting the shorter axis in a
    // symmetric square cone. Keep a small guard for PCF and near/far precision.
    let guard = (max_slope - min_slope) * 0.02;
    min_slope -= guard;
    max_slope += guard;
    near = (near - 0.15).max(0.01);
    far += 0.15;
    Ok(Mat4::frustum_rh(
        min_slope.x * near,
        max_slope.x * near,
        min_slope.y * near,
        max_slope.y * near,
        near,
        far,
    ) * view)
}

pub struct ShadowMap {
    pub map: Texture,
    uniform: Buffer,
    pub(crate) sample_layout: wgpu::BindGroupLayout,
    pub(crate) sample_binding: wgpu::BindGroup,
    pub(crate) depth_layout: wgpu::BindGroupLayout,
    pub(crate) depth_binding: wgpu::BindGroup,
    strength: f32,
}

impl ShadowMap {
    pub fn new(device: &Device, size: u32, matrix: Mat4, strength: f32) -> BelfastResult<Self> {
        let map = Texture::create_2d(
            device,
            size,
            size,
            TextureOptions::shadow_map("MagnoliaShadow"),
        )?;
        let uniform = Buffer::create(device, 80, BufferUsage::uniform(), "MagnoliaShadowUniform");
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
                    label: Some("ShadowSamplingLayout"),
                    entries: &[
                        uniform_entry(wgpu::ShaderStages::FRAGMENT),
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
                    ],
                });
        let sample_binding = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ShadowSamplingBinding"),
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
        // The depth pass never binds the map it is writing as a sampled texture.
        let depth_layout =
            device
                .gpu()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ShadowDepthLayout"),
                    entries: &[uniform_entry(wgpu::ShaderStages::VERTEX)],
                });
        let depth_binding = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ShadowDepthBinding"),
            layout: &depth_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.gpu().as_entire_binding(),
            }],
        });
        let shadow = Self {
            map,
            uniform,
            sample_layout,
            sample_binding,
            depth_layout,
            depth_binding,
            strength,
        };
        shadow.update_camera(device, matrix);
        Ok(shadow)
    }

    pub fn update_camera(&self, device: &Device, matrix: Mat4) {
        let mut values = matrix.to_cols_array().to_vec();
        // Strength, map size, comparison depth bias, world-space normal bias.
        values.extend([self.strength, self.map.width() as f32, 0.00008, 0.003]);
        self.uniform.write(device, &values, 0);
    }
}
