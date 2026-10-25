use crate::{
    fireflies::merged_depth,
    fog::{Fog, PARAMS_SHADER as FOG_PARAMS_SHADER},
    light_volume::{LightVolume, PARAMS_SHADER},
    sorting::SortCamera,
};
use belfast::Device;
use wgpu::util::DeviceExt;

/// Static forest color; Scene invalidates it together with forest occlusion.
pub struct ForestColor {
    pub view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    grade: wgpu::Buffer,
    inverse_camera: wgpu::Buffer,
}

impl ForestColor {
    pub fn new(
        device: &Device,
        lights: &LightVolume,
        fog: &Fog,
        depth: &wgpu::TextureView,
    ) -> Self {
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ForestColorLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: if merged_depth(device) { 6 } else { 7 },
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: if merged_depth(device) {
                                wgpu::TextureSampleType::Float { filterable: false }
                            } else {
                                wgpu::TextureSampleType::Depth
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 10,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let shader = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ForestColorCopyShader"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{PARAMS_SHADER}\n{FOG_PARAMS_SHADER}\n{}",
                        include_str!("forest-color.wgsl")
                    )
                    .into(),
                ),
            });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ForestColorCopyLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = device
            .gpu()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ForestColorCopyPipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if merged_depth(device) {
                        "fs_main"
                    } else {
                        "fs_depth"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[(
                            "srgb_attachment",
                            if device.format().is_srgb() { 1.0 } else { 0.0 },
                        )],
                        ..Default::default()
                    },
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
            });
        let grade = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ForestGrade"),
                contents: bytemuck::cast_slice(&[
                    [1.0f32, 1.0, 0.0, 0.0],
                    [0.055, 0.10, 0.26, 0.0],
                    [0.18, 0.30, 0.50, 0.0],
                    [0.025, 0.36, 0.41, 0.0],
                ]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let inverse_camera = device
            .gpu()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ForestInverseCamera"),
                contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let (view, bind_group) = Self::target(
            device,
            &layout,
            &grade,
            &inverse_camera,
            lights,
            fog,
            depth,
            [1, 1],
        );
        Self {
            view,
            bind_group,
            layout,
            pipeline,
            grade,
            inverse_camera,
        }
    }

    // Keep the shader's independent resources explicit when rebuilding the bind group.
    #[allow(clippy::too_many_arguments)]
    fn target(
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        grade: &wgpu::Buffer,
        inverse_camera: &wgpu::Buffer,
        lights: &LightVolume,
        fog: &Fog,
        depth: &wgpu::TextureView,
        size: [u32; 2],
    ) -> (wgpu::TextureView, wgpu::BindGroup) {
        let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
            label: Some("ForestColorCache"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: device.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind_group = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ForestColorCopyBindGroup"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grade.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&lights.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&lights.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: lights.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: inverse_camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: if merged_depth(device) { 6 } else { 7 },
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&fog.view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&fog.distance),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: fog.params.as_entire_binding(),
                },
            ],
        });
        (view, bind_group)
    }

    pub fn resize(
        &mut self,
        device: &Device,
        size: [u32; 2],
        lights: &LightVolume,
        fog: &Fog,
        depth: &wgpu::TextureView,
    ) {
        (self.view, self.bind_group) = Self::target(
            device,
            &self.layout,
            &self.grade,
            &self.inverse_camera,
            lights,
            fog,
            depth,
            size,
        );
    }

    pub fn update_camera(&self, device: &Device, camera: &SortCamera) {
        let inverse = (camera.projection * camera.view).inverse().to_cols_array();
        device
            .queue()
            .write_buffer(&self.inverse_camera, 0, bytemuck::cast_slice(&inverse));
    }

    pub fn set_grade(
        &self,
        device: &Device,
        strength: f32,
        brightness: f32,
        colors: &[f32],
    ) -> Result<(), String> {
        if !(0.0..=1.0).contains(&strength)
            || !(0.0..=2.0).contains(&brightness)
            || colors.len() != 9
            || colors.iter().any(|c| !(0.0..=1.0).contains(c))
        {
            return Err("Invalid forest color settings".into());
        }
        let values = [
            [strength, brightness, 0.0, 0.0],
            [colors[0], colors[1], colors[2], 0.0],
            [colors[3], colors[4], colors[5], 0.0],
            [colors[6], colors[7], colors[8], 0.0],
        ];
        device
            .queue()
            .write_buffer(&self.grade, 0, bytemuck::cast_slice(&values));
        Ok(())
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
