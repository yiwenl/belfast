use belfast::Geom;

#[test]
fn creates_xy_plane_geometry_with_positions_uvs_normals_and_indices() {
    let plane = Geom::plane(2.0, 4.0);

    assert_eq!(
        plane.positions,
        vec![-1.0, -2.0, 0.0, 1.0, -2.0, 0.0, 1.0, 2.0, 0.0, -1.0, 2.0, 0.0]
    );
    assert_eq!(plane.uvs, vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        plane.normals,
        vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(plane.indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
fn creates_cube_geometry_with_six_faces_and_unit_normals() {
    let cube = Geom::cube(2.0);
    let vertex_count = cube.positions.len() / 3;

    assert_eq!(vertex_count, 24);
    assert_eq!(cube.uvs.len(), vertex_count * 2);
    assert_eq!(cube.normals.len(), cube.positions.len());
    assert_eq!(cube.indices.len(), 36);
    assert!(cube
        .indices
        .iter()
        .all(|&index| index < vertex_count as u32));

    for chunk in cube.positions.as_chunks::<3>().0 {
        assert!(chunk.iter().all(|value| value.abs() <= 1.0));
        assert!(chunk.iter().any(|value| value.abs() == 1.0));
    }

    for chunk in cube.normals.as_chunks::<3>().0 {
        let length = (chunk[0] * chunk[0] + chunk[1] * chunk[1] + chunk[2] * chunk[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-6);
        assert_eq!(chunk.iter().filter(|value| value.abs() > 0.0).count(), 1);
    }

    for face in 0..6 {
        let base = face * 4;
        let expected = &cube.normals[base * 3..base * 3 + 3];
        for vertex in 1..4 {
            let start = (base + vertex) * 3;
            assert_eq!(&cube.normals[start..start + 3], expected);
        }
        let index_base = face * 6;
        assert_eq!(
            &cube.indices[index_base..index_base + 6],
            &[
                base as u32,
                base as u32 + 1,
                base as u32 + 2,
                base as u32,
                base as u32 + 2,
                base as u32 + 3
            ]
        );
    }
}

#[test]
fn cube_face_winding_matches_outward_normals() {
    let cube = Geom::cube(1.0);
    for face in 0..6 {
        let base = face * 4;
        let p0 = vertex(&cube.positions, base);
        let p1 = vertex(&cube.positions, base + 1);
        let p2 = vertex(&cube.positions, base + 2);
        let normal = vertex(&cube.normals, base);
        let edge0 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let edge1 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let cross = [
            edge0[1] * edge1[2] - edge0[2] * edge1[1],
            edge0[2] * edge1[0] - edge0[0] * edge1[2],
            edge0[0] * edge1[1] - edge0[1] * edge1[0],
        ];
        let alignment = cross[0] * normal[0] + cross[1] * normal[1] + cross[2] * normal[2];
        assert!(
            alignment > 0.0,
            "face {face} winding does not match outward normal {normal:?}"
        );
    }
}

fn vertex(values: &[f32], index: usize) -> [f32; 3] {
    let start = index * 3;
    [values[start], values[start + 1], values[start + 2]]
}
