use belfast::{
    Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh, VertexAttributeDescriptor,
    VertexBufferBinding, VertexBufferLayoutDescriptor,
};

const SHADER: &str = r#"
@vertex
fn vs_main(@location(0) position: vec2f) -> @builtin(position) vec4f {
    return vec4f(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

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

#[test]
fn draw_tracks_creator_device_and_mesh_layout() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("skipping GPU-backed Draw identity test: adapter unavailable");
        return;
    };
    let (gpu, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request first device");
    let (other_gpu, other_queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request second device");
    let device = Device::from_wgpu(gpu, queue, wgpu::TextureFormat::Rgba8UnormSrgb);
    let other_device =
        Device::from_wgpu(other_gpu, other_queue, wgpu::TextureFormat::Rgba8UnormSrgb);
    let cloned_device = device.clone();
    assert!(device.is_same(&cloned_device));
    assert!(!device.is_same(&other_device));

    let make_mesh = |format: wgpu::VertexFormat| {
        Mesh::new(3)
            .unwrap()
            .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
                array_stride: format.size(),
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format,
                    offset: 0,
                }],
                slot: None,
                step_mode: None,
            })
            .unwrap()
    };
    let mesh = make_mesh(wgpu::VertexFormat::Float32x2);
    let matching_mesh = make_mesh(wgpu::VertexFormat::Float32x2);
    let incompatible_mesh = make_mesh(wgpu::VertexFormat::Float32x3);
    let draw = Draw::new(
        &device,
        SHADER,
        &mesh,
        DrawOptions::new("IdentityTest", device.format()),
    );

    assert!(draw.device().is_same(&device));
    assert!(draw.is_compatible_mesh(&matching_mesh));
    assert!(!draw.is_compatible_mesh(&incompatible_mesh));
    assert_eq!(
        draw.validate_for_render(&other_device, &matching_mesh)
            .unwrap_err()
            .to_string(),
        "draw was created by a different device"
    );
    assert_eq!(
        draw.validate_for_render(&device, &incompatible_mesh)
            .unwrap_err()
            .to_string(),
        "mesh layout is incompatible with this draw"
    );

    let foreign_buffer = Buffer::from_data(
        &other_device,
        &[0.0_f32; 6],
        BufferUsage::vertex(),
        "ForeignPositions",
    );
    let foreign_mesh = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer(VertexBufferBinding {
            buffer: foreign_buffer,
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        })
        .unwrap();
    assert_eq!(
        Draw::validate_mesh_device(&device, &foreign_mesh)
            .unwrap_err()
            .to_string(),
        "mesh contains resources created by a different device"
    );
    assert_eq!(
        draw.validate_for_render(&device, &foreign_mesh)
            .unwrap_err()
            .to_string(),
        "mesh contains resources created by a different device"
    );
}
