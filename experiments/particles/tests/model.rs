use belfast_experiment_particles::model_data::ModelData;
use glam::Vec3;

fn magnolia() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/public/magnolia-branches.glb"
    ))
    .unwrap()
}

#[test]
fn loads_actual_magnolia_geometry_textures_and_grounded_placement() {
    let mut data = ModelData::parse(&magnolia()).unwrap();
    assert_eq!(data.primitives.len(), 1);
    let mesh = &data.primitives[0];
    assert_eq!(mesh.vertices.len(), 38_624);
    assert_eq!(mesh.indices.len(), 173_556);
    assert_eq!(mesh.base_color.width, 512);
    assert_eq!(mesh.normal_map.width, 512);
    assert!((mesh.normal_scale - 0.75).abs() < 1e-6);
    assert!(mesh.double_sided);
    let base = Vec3::new(0.05, -0.80, -5.98);
    data.place(base, 2.8).unwrap();
    let (min, max) = data.bounds();
    assert!((min.y - base.y).abs() < 1e-5);
    assert!((max.y - min.y - 2.8).abs() < 1e-5);
    assert!(((min.x + max.x) * 0.5 - base.x).abs() < 1e-5);
    assert!(((min.z + max.z) * 0.5 - base.z).abs() < 1e-5);
    assert!(data.primitives[0]
        .vertices
        .iter()
        .all(|v| { (Vec3::from(v.normal).length() - 1.).abs() < 1e-4 }));
}

#[test]
fn rejects_invalid_or_truncated_glb_without_panicking() {
    assert!(ModelData::parse(b"not a model").is_err());
    let mut bytes = magnolia();
    bytes.truncate(bytes.len() - 10);
    assert!(ModelData::parse(&bytes).is_err());
}

#[test]
fn model_shader_validates() {
    let module =
        naga::front::wgsl::parse_str(&belfast_experiment_particles::model::shader_source())
            .unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
fn rejects_wrong_accessor_component_types_without_panicking() {
    for (original, invalid) in [
        (
            "\"bufferView\":0,\"componentType\":5126",
            "\"bufferView\":0,\"componentType\":5122",
        ),
        (
            "\"bufferView\":1,\"componentType\":5126",
            "\"bufferView\":1,\"componentType\":5122",
        ),
        (
            "\"bufferView\":2,\"componentType\":5126",
            "\"bufferView\":2,\"componentType\":5122",
        ),
        (
            "\"bufferView\":3,\"componentType\":5123",
            "\"bufferView\":3,\"componentType\":5122",
        ),
    ] {
        let mut bytes = magnolia();
        let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json = std::str::from_utf8(&bytes[20..20 + json_length]).unwrap();
        assert!(json.contains(original));
        let changed = json.replacen(original, invalid, 1);
        bytes[20..20 + json_length].copy_from_slice(changed.as_bytes());
        assert!(
            ModelData::parse(&bytes).is_err(),
            "Invalid accessor: {invalid}"
        );
    }
}

#[test]
#[ignore = "requires a GPU; checks opaque mesh / transparent splat compositing"]
fn gpu_mesh_depth_and_point_light_response() {
    use belfast::{Device, DeviceOptions};
    use belfast_experiment_particles::{
        model::{depth_view, ModelRenderer, PointLight, DEPTH_FORMAT},
        model_data::{ImageData, PrimitiveData, Vertex},
        ply::GaussianCloud,
        sorting::SortCamera,
        splat::SplatRenderer,
    };
    use glam::Mat4;
    let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
        format: wgpu::TextureFormat::Rgba8Unorm,
        ..Default::default()
    }))
    .unwrap();
    let solid = |rgba: [u8; 4]| ImageData {
        width: 1,
        height: 1,
        rgba: rgba.to_vec(),
    };
    let mesh = ModelData {
        primitives: vec![PrimitiveData {
            vertices: [[-2., -2., -3.], [2., -2., -3.], [0., 2., -3.]]
                .into_iter()
                .map(|position| Vertex {
                    position,
                    normal: [0., 0., 1.],
                    uv: [0., 0.],
                })
                .collect(),
            indices: vec![0, 1, 2],
            base_color: solid([0, 255, 0, 255]),
            normal_map: solid([128, 128, 255, 255]),
            base_factor: [1.; 4],
            normal_scale: 0.,
            double_sided: true,
        }],
    };
    let model = ModelRenderer::new(&device, &mesh).unwrap();
    let camera = SortCamera::new(
        Mat4::IDENTITY,
        Mat4::perspective_rh(1., 1., 0.02, 100.),
        [64, 64],
    );
    model.update_camera(&device, &camera.projection.to_cols_array());
    let half_one = u32::from(half::f16::from_f32(1.).to_bits());
    let mut cloud = GaussianCloud {
        source_count: 2,
        positions: vec![[0., 0., -4., 0.25], [0., 0., -2., 0.25]],
        attributes: vec![
            [half_one, 0, half_one, u32::from_le_bytes([255, 0, 0, 220])],
            [half_one, 0, half_one, u32::from_le_bytes([0, 0, 255, 220])],
        ],
        sort_positions: Vec::new(),
    };
    // Covariance XX/XY, XZ/YY, YZ/ZZ: use unit XY and a zero Z variance.
    for a in &mut cloud.attributes {
        a[1] = half_one << 16;
        a[2] = 0;
    }
    let mut splats =
        SplatRenderer::new_with_depth(&device, &mut cloud, Some(DEPTH_FORMAT)).unwrap();
    splats.update_camera(&device, &camera);
    let size = wgpu::Extent3d {
        width: 64,
        height: 64,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("ModelDepthTest"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = texture.create_view(&Default::default());
    let depth = depth_view(&device, [64, 64]);
    let render = |splats: &SplatRenderer| {
        let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ModelDepthTest"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    stencil_ops: None,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            model.draw(&mut pass);
            splats.draw(&mut pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            size,
        );
        device.queue().submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .gpu()
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = readback.slice(..).get_mapped_range().unwrap();
        <[u8; 4]>::try_from(&bytes[32 * 256 + 32 * 4..32 * 256 + 32 * 4 + 4]).unwrap()
    };
    let only_model = render(&splats);
    // The mesh remains visible under ambient light; depth checks below compare
    // against this baseline rather than assuming the old directional fill.
    assert!(
        only_model[1] > 0 && only_model[0] == 0 && only_model[2] == 0,
        "{only_model:?}"
    );
    splats.set_indices(&device, &[0]).unwrap();
    assert_eq!(
        render(&splats),
        only_model,
        "The far red splat must be hidden by the mesh"
    );
    splats.set_indices(&device, &[0, 1]).unwrap();
    let front = render(&splats);
    // Check compositing, independent of the scene's editable exposure grade.
    assert!(
        front[1] > 0 && front[1] < only_model[1] && front[2] > only_model[2] && front[2] > front[0],
        "The near blue splat must blend over the mesh: {front:?}"
    );
    splats.set_indices(&device, &[1]).unwrap();
    assert_eq!(
        render(&splats),
        front,
        "The hidden far splat must not leak through the mesh"
    );

    splats.set_indices(&device, &[]).unwrap();
    let light = PointLight {
        position: [0., 0., -1.],
        color: [1.; 3],
        intensity: 1.,
    };
    model.update_light(&device, light);
    let near_light = render(&splats);
    model.update_light(
        &device,
        PointLight {
            position: [0., 0., 20.],
            ..light
        },
    );
    let far_light = render(&splats);
    assert!(
        near_light[1] > far_light[1] + 10,
        "Point light must weaken with distance: {near_light:?} vs {far_light:?}"
    );
    assert!(near_light[1] > only_model[1]);
    model.update_light(
        &device,
        PointLight {
            position: [0., 0., -5.],
            ..light
        },
    );
    assert_eq!(
        render(&splats),
        only_model,
        "A light behind the normal must not add diffuse light"
    );
    model.update_light(
        &device,
        PointLight {
            intensity: 0.,
            ..light
        },
    );
    assert_eq!(
        render(&splats),
        only_model,
        "Turning off the point light must retain the existing lighting"
    );
}
