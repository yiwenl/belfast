use belfast::{
    fluid::{FluidSettings, FluidSimulation, FluidSplat},
    Device,
};

#[test]
fn fluid_settings_reject_non_finite_and_out_of_range_values() {
    let mut settings = FluidSettings::default();
    assert!(settings.validate().is_ok());
    settings.velocity_dissipation = f32::NAN;
    assert!(settings.validate().is_err());
    settings.velocity_dissipation = 1.01;
    assert!(settings.validate().is_err());
    settings.velocity_dissipation = 0.985;
    settings.pressure_iterations = 0;
    assert!(settings.validate().is_err());
}

fn read_field(device: &Device, fluid: &FluidSimulation) -> Vec<[f32; 4]> {
    let output = fluid.field();
    let buffer = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: output.size(),
        mapped_at_creation: false,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    });
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(output.gpu(), 0, &buffer, 0, output.size());
    device.queue().submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().unwrap();
    bytemuck::cast_slice(&mapped).to_vec()
}

#[test]
#[ignore = "requires a native GPU adapter"]
fn gpu_fluid_injects_advects_and_resets_for_odd_and_even_iterations() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    assert!(FluidSimulation::new(&device, 0, FluidSettings::default()).is_err());
    assert!(FluidSimulation::new(&device, u32::MAX, FluidSettings::default()).is_err());
    let mut fluid = FluidSimulation::new(&device, 32, FluidSettings::default()).unwrap();
    let splat = FluidSplat {
        position: [0.5, 0.5],
        velocity: [1.0, 0.3],
        radius: 0.15,
        density: 0.5,
    };
    for iterations in [19, 20] {
        let settings = FluidSettings {
            pressure_iterations: iterations,
            ..Default::default()
        };
        fluid.set_settings(settings).unwrap();
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        assert!(fluid
            .step(&device, &mut encoder, f32::NAN, &[splat])
            .is_err());
        fluid
            .step(&device, &mut encoder, 1.0 / 60.0, &[splat])
            .unwrap();
        device.queue().submit([encoder.finish()]);
        let values = read_field(&device, &fluid);
        assert!(values.iter().flatten().all(|v| v.is_finite()));
        assert!(values.iter().any(|v| v[0].abs() > 0.01 && v[3] > 0.01));
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        fluid.reset(&mut encoder);
        device.queue().submit([encoder.finish()]);
        assert!(read_field(&device, &fluid)
            .iter()
            .flatten()
            .all(|v| *v == 0.0));
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        fluid.step(&device, &mut encoder, 1.0 / 60.0, &[]).unwrap();
        device.queue().submit([encoder.finish()]);
        assert!(read_field(&device, &fluid)
            .iter()
            .flatten()
            .all(|v| *v == 0.0));
    }
}
