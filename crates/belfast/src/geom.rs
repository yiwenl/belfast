#[derive(Clone, Debug, PartialEq)]
pub struct GeometryData {
    pub positions: Vec<f32>,
    pub uvs: Vec<f32>,
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
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }
}
