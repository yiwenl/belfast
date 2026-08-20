use belfast::{
    BelfastError, BindGroup, Buffer, BufferUsage, Compute, ComputeOptions, Device, UniformBlock,
    UniformFieldType,
};

#[test]
fn compute_dispatches_storage_buffer_writes() {
    let Some(device) = create_optional_device() else {
        return;
    };

    let shader = include_str!("../examples/shaders/compute.wgsl");
    let bind_group_layout =
        device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ComputeTestBindGroupLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
    let pipeline_layout = device
        .gpu()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ComputeTestLayout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
    let compute = Compute::new(
        &device,
        shader,
        ComputeOptions {
            label: "ComputeTest",
            layout: Some(&pipeline_layout),
            entry_point: "cs_main",
        },
    );

    let mut uniforms =
        UniformBlock::create([("time", UniformFieldType::F32)]).expect("compute uniform schema");
    uniforms.set_f32("time", 1.25).expect("set time");
    let uniform_buffer = Buffer::create(
        &device,
        uniforms.byte_size() as u64,
        BufferUsage::uniform(),
        "ComputeUniform",
    );
    uniform_buffer.write(&device, uniforms.bytes(), 0);
    let storage_buffer = Buffer::from_data(
        &device,
        &[0.0_f32],
        BufferUsage::storage(),
        "ComputeStorage",
    );
    let bind_group = BindGroup::create(
        &device,
        &compute.get_bind_group_layout(0),
        &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.gpu().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage_buffer.gpu().as_entire_binding(),
            },
        ],
        "ComputeBindGroup",
    );

    let mut encoder = device
        .gpu()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ComputeTestEncoder"),
        });
    compute.run(&mut encoder, Some(&bind_group), [1, 1, 1], Some("compute"));
    device.queue().submit([encoder.finish()]);
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll device");
}

fn create_optional_device() -> Option<Device> {
    match pollster::block_on(Device::create_headless()) {
        Ok(device) => Some(device),
        Err(BelfastError::AdapterUnavailable) => {
            eprintln!("skipping compute test: adapter unavailable");
            None
        }
        Err(error) => panic!("failed to create headless device: {error}"),
    }
}
