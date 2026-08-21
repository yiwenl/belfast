#[derive(Clone, Debug, PartialEq)]
pub struct GeometryData {
    pub positions: Vec<f32>,
    pub uvs: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
}

pub struct Geom;

impl Geom {
    pub fn plane(width: f32, height: f32) -> GeometryData {
        let half_width = width * 0.5;
        let half_height = height * 0.5;

        GeometryData {
            positions: vec![
                -half_width,
                -half_height,
                0.0,
                half_width,
                -half_height,
                0.0,
                half_width,
                half_height,
                0.0,
                -half_width,
                half_height,
                0.0,
            ],
            uvs: vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    pub fn cube(size: f32) -> GeometryData {
        let h = size * 0.5;
        let mut positions = Vec::with_capacity(72);
        let mut uvs = Vec::with_capacity(48);
        let mut normals = Vec::with_capacity(72);
        let mut indices = Vec::with_capacity(36);
        let mut base = 0;

        let mut push_face =
            |p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], n: [f32; 3]| {
                positions.extend_from_slice(&p0);
                positions.extend_from_slice(&p1);
                positions.extend_from_slice(&p2);
                positions.extend_from_slice(&p3);
                uvs.extend_from_slice(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
                for _ in 0..4 {
                    normals.extend_from_slice(&n);
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                base += 4;
            };

        push_face(
            [h, -h, -h],
            [h, h, -h],
            [h, h, h],
            [h, -h, h],
            [1.0, 0.0, 0.0],
        );
        push_face(
            [-h, -h, h],
            [-h, h, h],
            [-h, h, -h],
            [-h, -h, -h],
            [-1.0, 0.0, 0.0],
        );
        push_face(
            [-h, h, -h],
            [-h, h, h],
            [h, h, h],
            [h, h, -h],
            [0.0, 1.0, 0.0],
        );
        push_face(
            [-h, -h, h],
            [-h, -h, -h],
            [h, -h, -h],
            [h, -h, h],
            [0.0, -1.0, 0.0],
        );
        push_face(
            [-h, -h, h],
            [h, -h, h],
            [h, h, h],
            [-h, h, h],
            [0.0, 0.0, 1.0],
        );
        push_face(
            [h, -h, -h],
            [-h, -h, -h],
            [-h, h, -h],
            [h, h, -h],
            [0.0, 0.0, -1.0],
        );

        GeometryData {
            positions,
            uvs,
            normals,
            indices,
        }
    }
}
