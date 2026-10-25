use belfast::{BallHelper, BallHelperOptions, BelfastError, Device};

#[test]
fn ball_helper_rejects_invalid_params() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device
        .gpu()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BallTestPipelineLayout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let result = BallHelper::new(
            &device,
            BallHelperOptions {
                radius,
                ..BallHelperOptions::new(device.format(), &pipeline_layout)
            },
        );

        assert!(
            matches!(result, Err(BelfastError::InvalidBallParams)),
            "expected radius {radius:?} to be rejected"
        );
    }
}

#[test]
fn ball_helper_builds_with_a_compatible_camera_layout() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device
        .gpu()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BallTestPipelineLayout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

    let error_scope = device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
    let helper = BallHelper::new(
        &device,
        BallHelperOptions::new(device.format(), &pipeline_layout),
    );
    let updated =
        helper
            .as_ref()
            .expect("ball helper")
            .set([1.0, 2.0, 3.0], 0.5, [1.0, 0.2, 0.1, 0.8]);
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll device");
    let error = pollster::block_on(error_scope.pop());

    assert!(helper.is_ok());
    assert!(updated.is_ok());
    assert!(error.is_none(), "{}", error.unwrap());
}

fn camera_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
    device
        .gpu()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BallTestCameraLayout"),
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
            eprintln!("skipping GPU-backed ball helper test: adapter unavailable");
            None
        }
        Err(error) => panic!("failed to create headless device: {error}"),
    }
}
