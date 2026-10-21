use belfast::{Device, DeviceOptions};
use belfast_experiment_firefly_forest::scene::Scene;

#[test]
#[ignore = "requires a GPU; run explicitly for scene changes"]
fn initial_view_starts_inside_the_scan() {
    let device = pollster::block_on(Device::create_headless_with_options(DeviceOptions {
        format: wgpu::TextureFormat::Rgba8Unorm,
        ..Default::default()
    }))
    .unwrap();
    let scene = Scene::new(&device).unwrap();
    assert_eq!(scene.camera.position(), [0., 0., 5.]);
    assert_eq!(scene.camera.look_at_target(), [0., 0., 0.]);
}
