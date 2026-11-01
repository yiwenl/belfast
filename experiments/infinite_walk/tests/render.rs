use belfast::{Device, DeviceOptions};
use belfast_experiment_infinite_walk::scene::Scene;

#[test]
#[ignore = "requires GPU and the local birdsong PLY asset"]
fn real_cloud_moves_and_repeats_identically_in_both_directions() {
    let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
        format: wgpu::TextureFormat::Rgba8Unorm,
        ..Default::default()
    }))
    .unwrap();
    let scope = device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
    let mut scene = Scene::new(&device).unwrap();
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/public/forest.ply")).unwrap();
    assert_eq!(scene.load_point_cloud(&device, &bytes).unwrap(), 370_681);
    scene.resize(&device, 128, 64, 1.0);
    scene.camera.set_aspect(2.0).unwrap();
    scene.update(&device, 0.0, 0.0);
    let start = render_pixels(&device, &scene);
    assert!(
        start
            .chunks_exact(4)
            .filter(|p| p[0] != p[1] || p[1] != p[2])
            .count()
            > 100,
        "the real point cloud must cover the image with colored points"
    );

    let advance = |scene: &mut Scene, pixels: f32| {
        scene.walk.scroll(pixels);
        for _ in 0..240 {
            scene.update(&device, 1.0 / 60.0, 0.0);
        }
        render_pixels(&device, scene)
    };
    let forward = advance(&mut scene, 500.0);
    assert_ne!(forward, start, "scroll must move the viewpoint");
    assert_eq!(
        advance(&mut scene, 2000.0),
        forward,
        "one forward lap must repeat"
    );
    assert_eq!(
        advance(&mut scene, -4000.0),
        forward,
        "two backward laps must repeat"
    );
    assert_eq!(
        advance(&mut scene, -500.0),
        start,
        "reversing must return to the start"
    );
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

fn render_pixels(device: &Device, scene: &Scene) -> Vec<u8> {
    let size = wgpu::Extent3d {
        width: 128,
        height: 64,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("ViewTest"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: Some("ViewReadback"),
        size: 64 * 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    scene.render(device, &texture.create_view(&Default::default()));
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
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
                bytes_per_row: Some(512),
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
    let bytes = readback.slice(..).get_mapped_range().unwrap().to_vec();
    bytes
}
