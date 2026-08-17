use belfast::{BelfastError, Device};

#[test]
fn native_example_shaders_pass_wgpu_validation() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let shaders = [
        (
            "triangle",
            include_str!("../examples/shaders/triangle.wgsl"),
        ),
        (
            "colored_triangle",
            include_str!("../examples/shaders/colored_triangle.wgsl"),
        ),
        (
            "camera_uniform",
            include_str!("../examples/shaders/camera_uniform.wgsl"),
        ),
        ("texture", include_str!("../examples/shaders/texture.wgsl")),
        (
            "render_target_source",
            include_str!("../examples/shaders/render_target_source.wgsl"),
        ),
        (
            "render_target_present",
            include_str!("../examples/shaders/render_target_present.wgsl"),
        ),
        (
            "axis_helper",
            include_str!("../src/helpers/axis_helper.wgsl"),
        ),
    ];

    for (label, source) in shaders {
        device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
        let _module = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        device.gpu().poll(wgpu::Maintain::Wait);
        let error = pollster::block_on(device.gpu().pop_error_scope());

        assert!(error.is_none(), "{label}: {}", error.unwrap());
    }
}

fn create_optional_device() -> Option<Device> {
    match pollster::block_on(Device::create_headless()) {
        Ok(device) => Some(device),
        Err(BelfastError::AdapterUnavailable) => {
            eprintln!("skipping shader validation test: adapter unavailable");
            None
        }
        Err(error) => panic!("failed to create headless device: {error}"),
    }
}
