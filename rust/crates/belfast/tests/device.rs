use belfast::Device;

#[test]
fn wraps_an_existing_wgpu_device_and_format() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("skipping GPU-backed Device test: adapter unavailable");
        return;
    };
    let (gpu, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request device");

    let device = Device::from_wgpu(gpu, queue, wgpu::TextureFormat::Rgba8UnormSrgb);

    assert_eq!(device.format(), wgpu::TextureFormat::Rgba8UnormSrgb);
}
