use belfast::{UniformBlock, UniformFieldType};

#[test]
fn packs_uniform_fields_with_wgsl_alignment() {
    let block = UniformBlock::create([
        ("time", UniformFieldType::F32),
        ("mode", UniformFieldType::U32),
        ("direction", UniformFieldType::Vec3F),
        ("view_proj", UniformFieldType::Mat4x4F),
    ])
    .expect("schema is valid");

    assert_eq!(block.byte_size(), 96);
    assert_eq!(block.float_count(), 24);
    assert_eq!(block.get_offset("time").unwrap(), 0);
    assert_eq!(block.get_offset("mode").unwrap(), 1);
    assert_eq!(block.get_offset("direction").unwrap(), 4);
    assert_eq!(block.get_offset("view_proj").unwrap(), 8);
}

#[test]
fn writes_float_vector_matrix_and_u32_values_by_name() {
    let mut block = UniformBlock::create([
        ("mode", UniformFieldType::U32),
        ("color", UniformFieldType::Vec3F),
        ("transform", UniformFieldType::Mat4x4F),
    ])
    .expect("schema is valid");

    block.set_u32("mode", 7).unwrap();
    block.set_f32_slice("color", &[0.25, 0.5, 0.75]).unwrap();
    block
        .set_f32_slice(
            "transform",
            &[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 4.0, 1.0,
            ],
        )
        .unwrap();

    assert_eq!(block.u32_data()[0], 7);
    assert_eq!(&block.f32_data()[4..8], &[0.25, 0.5, 0.75, 0.0]);
    assert_eq!(&block.f32_data()[8..12], &[1.0, 0.0, 0.0, 0.0]);
    assert_eq!(&block.f32_data()[20..24], &[2.0, 3.0, 4.0, 1.0]);
}

#[test]
fn rejects_unknown_fields_and_short_values() {
    let mut block = UniformBlock::create([("color", UniformFieldType::Vec4F)]).unwrap();

    assert!(block.get_offset("missing").is_err());
    assert!(block.set_f32_slice("color", &[1.0, 0.0, 0.0]).is_err());
    assert!(block.set_u32("color", 1).is_err());
}
