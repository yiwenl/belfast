use belfast_experiment_firefly_forest::{
    ply::GaussianCloud,
    sorting::{SortCamera, SortGate, SplatSorter},
};
use glam::{Mat4, Vec3};

fn fixture(rows: &[[f32; 14]], reverse: bool) -> Vec<u8> {
    let names = [
        "x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "scale_0", "scale_1", "scale_2",
        "rot_0", "rot_1", "rot_2", "rot_3",
    ];
    let order: Vec<usize> = if reverse {
        (0..14).rev().collect()
    } else {
        (0..14).collect()
    };
    let mut header = format!(
        "ply\r\nformat binary_little_endian 1.0\r\nelement vertex {}\r\n",
        rows.len()
    );
    for &i in &order {
        header += &format!("property float {}\r\n", names[i]);
    }
    header += "end_header\r\n";
    let mut bytes = header.into_bytes();
    for row in rows {
        for &i in &order {
            bytes.extend(row[i].to_le_bytes());
        }
    }
    bytes
}

fn row() -> [f32; 14] {
    [
        1.,
        2.,
        3.,
        0.,
        0.,
        0.,
        0.,
        2.0_f32.ln(),
        0.,
        0.,
        1.,
        0.,
        0.,
        0.,
    ]
}

#[test]
fn parses_named_properties_and_decodes_gaussian_attributes() {
    let cloud = GaussianCloud::parse(&fixture(&[row()], true)).unwrap();
    assert_eq!(cloud.source_count, 1);
    assert_eq!(cloud.positions[0], [1., -2., -3., 4.]);
    assert_eq!(cloud.sort_positions[0], [1., -2., -3., 6.]);
    assert_eq!(
        cloud.attributes[0][3],
        u32::from_le_bytes([128, 128, 128, 128])
    );
    let unpack = |v: u32| {
        [
            half::f16::from_bits(v as u16).to_f32(),
            half::f16::from_bits((v >> 16) as u16).to_f32(),
        ]
    };
    assert_eq!(unpack(cloud.attributes[0][0]), [1., 0.]);
    assert_eq!(unpack(cloud.attributes[0][1]), [0., 0.25]);
    assert_eq!(unpack(cloud.attributes[0][2]), [0., 0.25]);
}

#[test]
fn rejects_truncation_invalid_quaternions_and_nonfinite_data() {
    let mut bytes = fixture(&[row()], false);
    bytes.pop();
    assert!(GaussianCloud::parse(&bytes)
        .unwrap_err()
        .contains("truncated"));
    let mut bad = row();
    bad[10] = 0.;
    assert!(GaussianCloud::parse(&fixture(&[bad], false)).is_err());
    bad = row();
    bad[0] = f32::NAN;
    assert!(GaussianCloud::parse(&fixture(&[bad], false)).is_err());
}

#[test]
fn removes_only_gaussians_below_fragment_alpha_cutoff() {
    let mut invisible = row();
    invisible[6] = -100.;
    let cloud = GaussianCloud::parse(&fixture(&[invisible, row()], false)).unwrap();
    assert_eq!(cloud.source_count, 2);
    assert_eq!(cloud.positions.len(), 1);
}

fn camera(x: f32, angle: f32) -> SortCamera {
    let eye = Vec3::new(x, 0., 0.);
    SortCamera::new(
        Mat4::look_at_rh(eye, eye + Vec3::new(angle.sin(), 0., -angle.cos()), Vec3::Y),
        Mat4::perspective_rh(1., 1., 0.01, 100.),
        [800, 800],
    )
}

#[test]
fn sorts_back_to_front_and_culls_without_losing_edge_splats() {
    let mut sorter = SplatSorter::new(vec![
        [0., 0., -2., 0.1],
        [0., 0., -8., 0.1],
        [0., 0., 2., 0.1],
        [100., 0., -2., 0.1],
        [1.3, 0., -2., 0.5],
    ]);
    assert_eq!(sorter.sort(&camera(0., 0.)), &[1, 0, 4]);
}

#[test]
fn skipped_motion_accumulates_and_projection_changes_invalidate_sort() {
    let mut gate = SortGate::default();
    assert!(gate.needs_sort(&camera(0., 0.)));
    gate.accept(camera(0., 0.));
    assert!(!gate.needs_sort(&camera(0.005, 0.001)));
    assert!(gate.needs_sort(&camera(0.021, 0.)));
    assert!(gate.needs_sort(&camera(0., 0.01)));
    let mut resized = camera(0., 0.);
    resized.viewport = [801, 800];
    assert!(gate.needs_sort(&resized));
}

#[test]
fn radix_sort_matches_float_order_across_depth_ranges() {
    let positions: Vec<_> = (0..1000)
        .map(|i| [0., 0., -0.1 - ((i * 719) % 1000) as f32 * 0.09, 0.01])
        .collect();
    let mut expected: Vec<_> = (0..positions.len() as u32).collect();
    expected.sort_by(|&a, &b| positions[a as usize][2].total_cmp(&positions[b as usize][2]));
    let mut sorter = SplatSorter::new(positions);
    assert_eq!(sorter.sort(&camera(0., 0.)), expected);
    assert_eq!(sorter.sort(&camera(0., 0.)), expected);
}

#[test]
fn frozen_order_keeps_culling_and_includes_newly_visible_splats() {
    let mut sorter = SplatSorter::new(vec![
        [0., 0., -2., 0.1],
        [0., 0., -8., 0.1],
        [0., 0., 2., 0.1],
        [0., 0., 8., 0.1],
    ]);
    sorter.set_frozen(true);
    assert_eq!(sorter.sort(&camera(0., 0.)), &[1, 0]);
    assert_eq!(sorter.sort_count(), 1);
    // These were behind the initial camera and absent from its visible list.
    assert_eq!(sorter.sort(&camera(0., std::f32::consts::PI)), &[2, 3]);
    assert_eq!(
        sorter.sort_count(),
        1,
        "Only culling should run after freezing"
    );
    assert!(!sorter.did_sort());
    assert_eq!(sorter.sort(&camera(0., 0.)), &[1, 0]);
    assert_eq!(sorter.sort_count(), 1);

    sorter.set_frozen(false);
    assert_eq!(sorter.sort(&camera(0., std::f32::consts::PI)), &[3, 2]);
    assert_eq!(sorter.sort_count(), 2);
    assert!(sorter.did_sort());
    sorter.set_frozen(true);
    assert_eq!(sorter.sort(&camera(0., std::f32::consts::PI)), &[3, 2]);
    assert_eq!(
        sorter.sort_count(),
        3,
        "Re-entering frozen mode captures a new camera"
    );
}

#[test]
fn frozen_culling_handles_empty_views_and_projection_changes() {
    let mut sorter = SplatSorter::new(vec![[1.5, 0., -2., 0.01], [0., 0., -8., 0.01]]);
    sorter.set_frozen(true);
    assert_eq!(sorter.sort(&camera(0., 0.)), &[1]);
    assert!(sorter.sort(&camera(100., 0.)).is_empty());
    let mut wide = camera(0., 0.);
    wide.projection = Mat4::perspective_rh(2., 1., 0.01, 100.);
    assert_eq!(sorter.sort(&wide), &[1, 0]);
    assert_eq!(sorter.sort_count(), 1);
}

#[test]
fn shader_parses_and_validates() {
    let module =
        naga::front::wgsl::parse_str(belfast_experiment_firefly_forest::splat::SHADER).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::SHADER_FLOAT16_IN_FLOAT32,
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "requires a GPU; run explicitly for renderer changes"]
fn gpu_draws_anisotropic_gaussians_in_back_to_front_order() {
    use belfast::{Device, DeviceOptions};
    use belfast_experiment_firefly_forest::splat::SplatRenderer;
    let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
        format: wgpu::TextureFormat::Rgba8Unorm,
        ..Default::default()
    }))
    .unwrap();
    let scene = belfast_experiment_firefly_forest::scene::Scene::new(&device).unwrap();
    assert_eq!(scene.camera.position(), [0., 0., 0.]);
    assert_eq!(scene.camera.look_at_target(), [0., 0., -1.]);
    let mut far = row();
    far[..3].copy_from_slice(&[0., 0., 4.]);
    far[3..6].copy_from_slice(&[1.772_453_8, -1.772_453_8, -1.772_453_8]);
    far[7..10].copy_from_slice(&[0.8_f32.ln(), 0.24_f32.ln(), 0.01_f32.ln()]);
    let mut near = far;
    near[2] = 2.;
    near[3..6].copy_from_slice(&[-1.772_453_8, -1.772_453_8, 1.772_453_8]);
    near[7] = 0.4_f32.ln();
    near[8] = 0.12_f32.ln();
    let mut cloud = GaussianCloud::parse(&fixture(&[near, far], false)).unwrap();
    let mut renderer = SplatRenderer::new(&device, &mut cloud).unwrap();
    renderer.set_indices(&device, &[1, 0]).unwrap();
    assert!(renderer.set_indices(&device, &[2]).is_err());
    let mut camera = camera(0., 0.);
    camera.viewport = [64, 64];
    renderer.update_camera(&device, &camera);
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("SplatTestTarget"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: Some("SplatTestReadback"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SplatTest"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        renderer.draw(&mut pass);
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
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );
    device.queue().submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = readback.slice(..).get_mapped_range().unwrap();
    let pixel = |x: usize, y: usize| &bytes[y * 256 + x * 4..y * 256 + x * 4 + 4];
    let center = pixel(32, 32);
    assert!(
        center[2] > center[0] * 3 / 2 && center[0] > 40 && center[1] == 0,
        "center: {center:?}"
    );
    assert!(
        pixel(40, 32)[2] > pixel(32, 40)[2] * 3,
        "expected a horizontal ellipse"
    );
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
}

#[test]
#[ignore = "large local asset; run explicitly when changing the loader"]
fn forest_load_and_sort() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/public/forest.ply")).unwrap();
    let start = std::time::Instant::now();
    let cloud = GaussianCloud::parse(&bytes).unwrap();
    assert_eq!(cloud.source_count, 4_320_000);
    println!(
        "Loaded {} active Gaussians in {:?}",
        cloud.positions.len(),
        start.elapsed()
    );
    let start = std::time::Instant::now();
    let mut sorter = SplatSorter::new(cloud.sort_positions);
    let indices = sorter.sort(&camera(0., 0.));
    assert!(!indices.is_empty());
    println!(
        "Sorted {} visible Gaussians in {:?}",
        indices.len(),
        start.elapsed()
    );
}
