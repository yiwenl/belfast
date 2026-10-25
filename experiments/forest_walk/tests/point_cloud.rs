use belfast_experiment_forest_walk::point_cloud::PointCloud;

fn ply(points: &[[f32; 3]]) -> Vec<u8> {
    let header = format!("ply\nformat binary_little_endian 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nproperty float nx\nproperty float ny\nproperty float nz\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n", points.len());
    let mut bytes = header.into_bytes();
    for p in points {
        for value in p.iter().chain([0., 1., 0.].iter()) {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([17, 128, 255]);
    }
    bytes
}

#[test]
fn retains_world_positions_and_srgb_colors_while_skipping_normals() {
    let cloud = PointCloud::parse(&ply(&[[1., -2., 3.], [-4., 5., -6.]])).unwrap();
    assert_eq!(cloud.points.len(), 2);
    assert_eq!(cloud.points[0].position, [1., -2., 3.]);
    assert_eq!(cloud.points[1].position, [-4., 5., -6.]);
    assert_eq!(cloud.points[0].color, [17, 128, 255, 255]);
}

#[test]
fn rejects_truncated_and_non_finite_vertices() {
    let mut bytes = ply(&[[1., 2., 3.]]);
    bytes.pop();
    assert!(PointCloud::parse(&bytes).unwrap_err().contains("truncated"));
    assert!(PointCloud::parse(&ply(&[[f32::NAN, 0., 0.]]))
        .unwrap_err()
        .contains("finite"));
}

#[test]
fn accepts_reordered_properties_and_crlf_headers() {
    let mut bytes = b"ply\r\nformat binary_little_endian 1.0\r\nelement vertex 1\r\nproperty uchar blue\r\nproperty float z\r\nproperty uchar red\r\nproperty float x\r\nproperty float y\r\nproperty uchar green\r\nend_header\r\n".to_vec();
    bytes.push(30);
    bytes.extend(3f32.to_le_bytes());
    bytes.push(10);
    bytes.extend(1f32.to_le_bytes());
    bytes.extend(2f32.to_le_bytes());
    bytes.push(20);
    let cloud = PointCloud::parse(&bytes).unwrap();
    assert_eq!(cloud.points[0].position, [1., 2., 3.]);
    assert_eq!(cloud.points[0].color, [10, 20, 30, 255]);
}

#[test]
fn rejects_missing_colors_empty_cloud_and_unsupported_format() {
    assert!(PointCloud::parse(b"ply\nformat ascii 1.0\nend_header\n").is_err());
    assert!(PointCloud::parse(&ply(&[])).is_err());
    let invalid = String::from_utf8(ply(&[]))
        .unwrap()
        .replace("property uchar red", "property uchar other");
    assert!(PointCloud::parse(invalid.as_bytes()).is_err());
}
