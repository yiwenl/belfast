//! Manual throughput diagnostic, separate from the browser's refresh-limited FPS.
use belfast::{Device, DeviceOptions};
use belfast_experiment_infinite_walk::scene::Scene;
use std::time::{Duration, Instant};

#[test]
#[ignore = "manual 1080p GPU benchmark; requires both local PLY assets"]
fn completed_gpu_frames_at_1080p() {
    let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
        format: wgpu::TextureFormat::Rgba16Float,
        ..Default::default()
    }))
    .unwrap();
    let mut scene = Scene::new(&device).unwrap();
    let root = env!("CARGO_MANIFEST_DIR");
    scene
        .load_point_cloud(
            &device,
            &std::fs::read(format!("{root}/public/forest.ply")).unwrap(),
        )
        .unwrap();
    scene
        .load_backdrop(
            &device,
            &std::fs::read(format!("{root}/public/backdrop.ply")).unwrap(),
        )
        .unwrap();
    scene.resize(&device, 1920, 1080, 1.0);
    scene.camera.set_aspect(1920.0 / 1080.0).unwrap();
    scene.walk.toggle_auto_backward();
    let target = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("1080p throughput benchmark"),
        size: wgpu::Extent3d {
            width: 1920,
            height: 1080,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut frame = 0;
    let mut draw = || {
        scene.update(&device, 1.0 / 60.0, f64::from(frame) * 1000.0 / 60.0);
        scene.simulate(&device, 1.0 / 60.0).unwrap();
        scene.render(&device, &view);
        // Include actual GPU completion, not just command submission or CPU time.
        device
            .gpu()
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        frame += 1;
    };
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_secs(3) {
        draw();
    }
    let start = Instant::now();
    let mut count = 0;
    while start.elapsed() < Duration::from_secs(20) {
        draw();
        count += 1;
    }
    let seconds = start.elapsed().as_secs_f64();
    println!("GPU_COMPLETED: frames={count}, seconds={seconds:.6}, fps={:.4}, resolution=1920x1080, format=rgba16float, includes=simulation+shadows+fog+backdrop", f64::from(count) / seconds);
}
