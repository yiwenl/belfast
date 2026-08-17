use belfast::{Mesh, MeshIndexFormat, VertexAttributeDescriptor, VertexBufferLayoutDescriptor};

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
            slot: Some(1),
            step_mode: None,
        })
        .unwrap();

    let layouts = mesh.vertex_layouts();

    assert_eq!(layouts.len(), 2);
    assert!(layouts[0].is_none());
    let layout = layouts[1].as_ref().unwrap();
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
fn tracks_index_buffer_metadata() {
    let mut mesh = Mesh::new(4).unwrap();

    mesh.set_index_buffer_metadata(6, MeshIndexFormat::Uint32)
        .unwrap();

    assert!(mesh.has_index_buffer());
    assert_eq!(mesh.index_count(), 6);
    assert_eq!(mesh.index_format(), MeshIndexFormat::Uint32);
}
