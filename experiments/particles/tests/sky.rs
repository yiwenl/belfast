use belfast_experiment_particles::sky::{ray_matrix, validate_image, SHADER};
use glam::{Mat4, Vec3};

#[test]
fn background_ignores_translation_but_tracks_rotation() {
    let projection = Mat4::perspective_rh(1., 1.5, 0.02, 200.);
    let first = ray_matrix(Mat4::IDENTITY, projection);
    let moved = ray_matrix(Mat4::from_translation(Vec3::new(-3., 4., 1.)), projection);
    assert!(first.abs_diff_eq(moved, 1e-6));
    let rotated = ray_matrix(Mat4::from_rotation_y(1.), projection);
    assert!(!first.abs_diff_eq(rotated, 1e-3));
    let ray = first * glam::Vec4::new(0., 0., 1., 1.);
    assert!(ray.truncate().normalize().abs_diff_eq(Vec3::NEG_Z, 1e-6));
}

#[test]
fn rejects_bad_panorama_dimensions_before_upload() {
    assert!(validate_image(8, 4, 128, 4096).is_ok());
    for (w, h, len) in [
        (0, 0, 0),
        (8, 4, 127),
        (4, 4, 64),
        (8192, 4096, 134217728),
        (u32::MAX, 1, 4),
    ] {
        assert!(validate_image(w, h, len, 4096).is_err());
    }
}

#[test]
fn sky_shader_validates() {
    let module = naga::front::wgsl::parse_str(SHADER).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "requires a GPU; run explicitly for background renderer changes"]
fn gpu_background_presets_panorama_and_color_space() {
    use belfast::{Device, DeviceOptions};
    use belfast_experiment_particles::sky::SkyRenderer;
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
            format,
            ..Default::default()
        }))
        .unwrap();
        let mut sky = SkyRenderer::new(&device);
        let projection = Mat4::perspective_rh(1., 1., 0.02, 200.);
        let render = |sky: &SkyRenderer, view: Mat4| {
            sky.update(&device, view, projection);
            let size = wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            };
            let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
                label: Some("SkyTest"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
                label: Some("SkyTestReadback"),
                size: 64 * 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let view_texture = texture.create_view(&Default::default());
            let mut encoder = device.gpu().create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("SkyTest"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view_texture,
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
                sky.draw(&mut pass);
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
                .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
            device
                .gpu()
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            rx.recv().unwrap().unwrap();
            let bytes = readback.slice(..).get_mapped_range().unwrap();
            bytes.to_vec()
        };
        let center = |bytes: &[u8]| -> [u8; 4] {
            bytes[32 * 256 + 32 * 4..32 * 256 + 32 * 4 + 4]
                .try_into()
                .unwrap()
        };
        let day = render(&sky, Mat4::IDENTITY);
        assert!(center(&day)[0] > 200);
        assert!(day.chunks_exact(4).all(|rgba| rgba[3] == 255));
        sky.set_preset("night").unwrap();
        assert!(center(&render(&sky, Mat4::IDENTITY))[0] < 30);
        assert!(sky.set_preset("image").is_err());
        let rgba: Vec<u8> = (0..32)
            .flat_map(|y| (0..64).flat_map(move |x| [x * 4, y * 8, 32, 255]))
            .collect();
        sky.set_image(&device, &rgba, 64, 32).unwrap();
        let front = render(&sky, Mat4::IDENTITY);
        let p = center(&front);
        assert!(
            (120..136).contains(&p[0]) && (120..136).contains(&p[1]) && p[2] == 32,
            "{format:?}: {p:?}"
        );
        assert_eq!(
            front,
            render(&sky, Mat4::from_translation(Vec3::new(8., -3., 2.)))
        );
        assert!((184..200).contains(
            &center(&render(
                &sky,
                Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2)
            ))[0]
        ));
        assert!(sky.set_image(&device, &[0; 4], 64, 32).is_err());
        assert_eq!(front, render(&sky, Mat4::IDENTITY));
        sky.set_preset("off").unwrap();
        assert!(render(&sky, Mat4::IDENTITY).iter().all(|v| *v == 0));
    }
}
