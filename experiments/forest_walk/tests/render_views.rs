use belfast::{Device, DeviceOptions};
use belfast_experiment_forest_walk::scene::Scene;

#[test]
#[ignore = "requires GPU; verifies independent views and instanced camera lines"]
fn overview_does_not_change_the_walk_render() {
    let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
        format: wgpu::TextureFormat::Rgba8Unorm,
        ..Default::default()
    }))
    .unwrap();
    let scope = device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
    let mut scene = Scene::new(&device).unwrap();
    let json = r#"{"cameras":[{"camera_id":1,"width":800,"height":800,"named_parameters":{"fy":400}}],"frames":[{"camera_id":1,"timestamp_seconds":0,"camera_to_world":[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]},{"camera_id":1,"timestamp_seconds":1,"camera_to_world":[[1,0,0,0.4],[0,1,0,0],[0,0,1,0],[0,0,0,1]]}]}"#;
    scene.load_cameras(&device, json).unwrap();
    let mut ply = b"ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n".to_vec();
    for point in [[0f32, 0., 1.], [0.25, 0.25, 1.], [-0.25, -0.25, 1.]] {
        for value in point {
            ply.extend(value.to_le_bytes());
        }
        ply.extend([255, 255, 255]);
    }
    scene.load_point_cloud(&device, &ply).unwrap();
    scene.resize_surface(&device, 128, 64);
    scene.set_view_origins([64, 0], [0, 0]);
    scene.resize(&device, 64, 64, 1.);
    scene.resize_overview(&device, 64, 64, 1.);
    scene.update(&device, 0., false);
    let walk = render_pixels(&device, &scene, false);
    let right = |pixels: &[u8]| -> Vec<u8> {
        pixels
            .as_chunks::<512>()
            .0
            .iter()
            .flat_map(|row| row[256..].iter().copied())
            .collect()
    };
    assert!(walk.as_chunks::<4>().0.iter().any(|p| p[0] == 255));
    for _ in 0..3 {
        scene.update(&device, 0., true);
        let overview = render_pixels(&device, &scene, true);
        assert!(
            overview
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[1] > 180 && p[0] < 100),
            "cyan recorded frustum"
        );
        assert!(
            overview
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 240 && p[1] > 100 && p[1] < 180),
            "orange current frustum"
        );
        assert_eq!(right(&overview), right(&walk));
    }
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

fn render_pixels(device: &Device, scene: &Scene, overview: bool) -> Vec<u8> {
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
    scene.render(device, &texture.create_view(&Default::default()), overview);
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
