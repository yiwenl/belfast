use belfast::{AxisHelper, AxisHelperOptions, BelfastError, Device};

#[test]
fn axis_helper_rejects_non_positive_length() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device
        .gpu()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("AxisTestPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
    let result = AxisHelper::new(
        &device,
        AxisHelperOptions {
            length: 0.0,
            ..AxisHelperOptions::new(device.format(), &pipeline_layout)
        },
    );

    assert!(matches!(result, Err(BelfastError::InvalidAxisLength)));
}

#[test]
fn axis_helper_builds_with_a_compatible_camera_layout() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device
        .gpu()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("AxisTestPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
    let helper = AxisHelper::new(
        &device,
        AxisHelperOptions::new(device.format(), &pipeline_layout),
    );
    device.gpu().poll(wgpu::Maintain::Wait);
    let error = pollster::block_on(device.gpu().pop_error_scope());

    assert!(helper.is_ok());
    assert!(error.is_none(), "{}", error.unwrap());
}

fn camera_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
    device
        .gpu()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("AxisTestCameraLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
}

fn create_optional_device() -> Option<Device> {
    match pollster::block_on(Device::create_headless()) {
        Ok(device) => Some(device),
        Err(BelfastError::AdapterUnavailable) => {
            eprintln!("skipping GPU-backed axis helper test: adapter unavailable");
            None
        }
        Err(error) => panic!("failed to create headless device: {error}"),
    }
}
