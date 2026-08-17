use belfast::Geom;

#[test]
fn creates_xy_plane_geometry_with_positions_uvs_and_indices() {
    let plane = Geom::plane(2.0, 4.0);

    assert_eq!(
        plane.positions,
        vec![-1.0, -2.0, 0.0, 1.0, -2.0, 0.0, 1.0, 2.0, 0.0, -1.0, 2.0, 0.0]
    );
    assert_eq!(plane.uvs, vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(plane.indices, vec![0, 1, 2, 0, 2, 3]);
}
