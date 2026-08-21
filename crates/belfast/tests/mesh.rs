use belfast::{
    Buffer, BufferUsage, Device, Mesh, MeshIndexFormat, VertexAttributeDescriptor,
    VertexBufferBinding, VertexBufferLayoutDescriptor,
};

fn create_test_devices() -> Option<(Device, Device)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    let (first_gpu, first_queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    let (second_gpu, second_queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

    Some((
        Device::from_wgpu(
            first_gpu,
            first_queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None,
        ),
        Device::from_wgpu(
            second_gpu,
            second_queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None,
        ),
    ))
}

#[test]
fn creates_vertex_layouts_with_explicit_slots() {
    let mesh = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        })
        .unwrap();

    let layouts = mesh.vertex_layouts();

    assert_eq!(layouts.len(), 1);
    let layout = layouts[0].as_ref().unwrap();
    assert_eq!(layout.array_stride, 8);
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);
    assert_eq!(layout.attributes[0].shader_location, 0);
}

#[test]
fn assigns_next_free_vertex_buffer_slot() {
    let mesh = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        })
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 12,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        })
        .unwrap();

    let layouts = mesh.vertex_layouts();

    assert!(layouts[0].is_some());
    assert!(layouts[1].is_some());
}

#[test]
fn rejects_duplicate_slots_and_empty_meshes() {
    assert!(Mesh::new(0).is_err());

    let result = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        })
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 12,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        });

    assert!(result.is_err());
}

#[test]
fn rejects_sparse_and_extreme_vertex_buffer_slots() {
    for slot in [1, u32::MAX] {
        let result = Mesh::new(3)
            .unwrap()
            .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
                array_stride: 8,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                }],
                slot: Some(slot),
                step_mode: None,
            });

        assert_eq!(
            result.err().unwrap().to_string(),
            format!("vertex buffer slot {slot} is not contiguous; expected slot 0")
        );
    }
}

#[test]
fn rejects_duplicate_shader_locations_across_bindings() {
    let within_binding =
        Mesh::new(3)
            .unwrap()
            .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
                array_stride: 16,
                attributes: vec![
                    VertexAttributeDescriptor {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                    },
                    VertexAttributeDescriptor {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 8,
                    },
                ],
                slot: None,
                step_mode: None,
            });
    assert_eq!(
        within_binding.err().unwrap().to_string(),
        "vertex attribute shader location 2 is already in use"
    );

    let result = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        })
        .unwrap()
        .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
            array_stride: 12,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        });

    assert_eq!(
        result.err().unwrap().to_string(),
        "vertex attribute shader location 2 is already in use"
    );
}

#[test]
fn rejects_invalid_vertex_buffer_stride_and_attribute_extents() {
    let cases = [
        (
            0,
            0,
            wgpu::VertexFormat::Float32x2,
            "vertex buffer array stride must be greater than zero",
        ),
        (
            10,
            0,
            wgpu::VertexFormat::Float32x2,
            "vertex buffer array stride 10 must be a multiple of 4",
        ),
        (
            12,
            2,
            wgpu::VertexFormat::Float32x2,
            "vertex attribute at shader location 0 has misaligned offset 2",
        ),
        (
            8,
            0,
            wgpu::VertexFormat::Float32x3,
            "vertex attribute at shader location 0 exceeds array stride 8",
        ),
    ];

    for (array_stride, offset, format, expected) in cases {
        let result = Mesh::new(3)
            .unwrap()
            .add_vertex_buffer_layout(VertexBufferLayoutDescriptor {
                array_stride,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format,
                    offset,
                }],
                slot: None,
                step_mode: None,
            });

        assert_eq!(result.err().unwrap().to_string(), expected);
    }
}

#[test]
fn validates_vertex_buffer_device_limits_and_extent() {
    let Some((device, other_device)) = create_test_devices() else {
        eprintln!("skipping GPU-backed mesh validation test: adapter unavailable");
        return;
    };

    let too_small = Buffer::from_data(&device, &[0.0_f32; 5], BufferUsage::vertex(), "TooSmall");
    let result = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer(VertexBufferBinding {
            buffer: too_small,
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        });
    assert_eq!(
        result.err().unwrap().to_string(),
        "vertex buffer at slot 0 requires at least 24 bytes for this mesh, got 20"
    );

    let valid = Buffer::from_data(&device, &[0.0_f32; 6], BufferUsage::vertex(), "Valid");
    let mut mesh = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer(VertexBufferBinding {
            buffer: valid,
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

    let foreign_index = Buffer::from_data(
        &other_device,
        &[0_u16; 3],
        BufferUsage::index(),
        "ForeignIndices",
    );
    assert_eq!(
        mesh.set_index_buffer(foreign_index, 3, MeshIndexFormat::Uint16)
            .err()
            .unwrap()
            .to_string(),
        "index buffer was created by a different device"
    );
    assert!(!mesh.has_index_buffer());

    let foreign = Buffer::from_data(
        &other_device,
        &[0.0_f32; 9],
        BufferUsage::vertex(),
        "Foreign",
    );
    let result = mesh.add_vertex_buffer(VertexBufferBinding {
        buffer: foreign,
        array_stride: 12,
        attributes: vec![VertexAttributeDescriptor {
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
        }],
        slot: None,
        step_mode: None,
    });
    assert_eq!(
        result.err().unwrap().to_string(),
        "vertex buffer at slot 1 was created by a different device"
    );

    let oversized_stride = u64::from(device.gpu().limits().max_vertex_buffer_array_stride) + 4;
    let buffer = Buffer::from_data(
        &device,
        &[0.0_f32; 2],
        BufferUsage::vertex(),
        "OversizedStride",
    );
    let result = Mesh::new(1)
        .unwrap()
        .add_vertex_buffer(VertexBufferBinding {
            buffer,
            array_stride: oversized_stride,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        });
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "vertex buffer array stride {oversized_stride} exceeds device limit {}",
            device.gpu().limits().max_vertex_buffer_array_stride
        )
    );

    let buffer = Buffer::from_data(
        &device,
        &[0.0_f32; 2],
        BufferUsage::vertex(),
        "ExcessiveLocation",
    );
    let location = device.gpu().limits().max_vertex_attributes;
    let result = Mesh::new(1)
        .unwrap()
        .add_vertex_buffer(VertexBufferBinding {
            buffer,
            array_stride: 8,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: location,
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        });
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "vertex attribute shader location {location} exceeds device limit {}",
            device.gpu().limits().max_vertex_attributes
        )
    );
}

#[test]
fn instance_buffer_extent_requires_one_element() {
    let Some((device, _)) = create_test_devices() else {
        eprintln!("skipping GPU-backed instance buffer test: adapter unavailable");
        return;
    };
    let buffer = Buffer::from_data(&device, &[0.0_f32; 3], BufferUsage::vertex(), "OneInstance");

    let result = Mesh::new(50)
        .unwrap()
        .add_vertex_buffer(VertexBufferBinding {
            buffer,
            array_stride: 12,
            attributes: vec![VertexAttributeDescriptor {
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
            }],
            slot: None,
            step_mode: Some(wgpu::VertexStepMode::Instance),
        });

    assert!(result.is_ok());
}

#[test]
fn layout_signature_is_canonical_and_detects_incompatible_meshes() {
    let position_layout = VertexBufferLayoutDescriptor {
        array_stride: 8,
        attributes: vec![VertexAttributeDescriptor {
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
        }],
        slot: None,
        step_mode: None,
    };
    let matching_layout = position_layout.clone();
    let different_layout = VertexBufferLayoutDescriptor {
        array_stride: 12,
        attributes: vec![VertexAttributeDescriptor {
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
        }],
        slot: None,
        step_mode: None,
    };
    let first = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer_layout(position_layout)
        .unwrap();
    let matching = Mesh::new(99)
        .unwrap()
        .add_vertex_buffer_layout(matching_layout)
        .unwrap();
    let different = Mesh::new(3)
        .unwrap()
        .add_vertex_buffer_layout(different_layout)
        .unwrap();

    assert_eq!(first.layout_signature(), matching.layout_signature());
    assert_ne!(first.layout_signature(), different.layout_signature());
}

#[test]
fn tracks_index_buffer_metadata() {
    let mut mesh = Mesh::new(4).unwrap();

    mesh.set_index_buffer_metadata(6, MeshIndexFormat::Uint32)
        .unwrap();

    assert!(mesh.has_index_buffer());
    assert_eq!(mesh.index_count(), 6);
    assert_eq!(mesh.index_format(), MeshIndexFormat::Uint32);
}
