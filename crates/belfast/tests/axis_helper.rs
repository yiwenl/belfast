use belfast::{AxisHelper, AxisHelperOptions, BelfastError, Device};

#[test]
fn axis_helper_rejects_invalid_lengths() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device
        .gpu()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("AxisTestPipelineLayout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
    for length in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let result = AxisHelper::new(
            &device,
            AxisHelperOptions {
                length,
                ..AxisHelperOptions::new(device.format(), &pipeline_layout)
            },
        );

        assert!(
            matches!(result, Err(BelfastError::InvalidAxisLength)),
            "expected {length:?} to be rejected"
        );
    }
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
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

    let error_scope = device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
    let helper = AxisHelper::new(
        &device,
        AxisHelperOptions::new(device.format(), &pipeline_layout),
    );
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll device");
    let error = pollster::block_on(error_scope.pop());

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
