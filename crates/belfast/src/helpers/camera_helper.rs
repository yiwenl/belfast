use glam::{Mat4, Vec4};

use crate::{
    BelfastError, BelfastResult, BindGroup, Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh,
    VertexAttributeDescriptor, VertexBufferBinding,
};

const VERTEX_COUNT: u32 = 24;
const POSITION_FLOATS: usize = VERTEX_COUNT as usize * 3;

const CLIP_CORNERS: [[f32; 3]; 8] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, -1.0, 0.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
];

const EDGES: [[usize; 2]; 12] = [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4],
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
];

pub struct CameraHelperOptions<'a> {
    pub label: &'a str,
    pub color: [f32; 3],
    pub format: wgpu::TextureFormat,
    pub layout: &'a wgpu::PipelineLayout,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> CameraHelperOptions<'a> {
    pub fn new(format: wgpu::TextureFormat, layout: &'a wgpu::PipelineLayout) -> Self {
        Self {
            label: "CameraHelper",
            color: [1.0, 1.0, 1.0],
            format,
            layout,
            depth_stencil: None,
        }
    }
}

pub struct CameraHelper {
    mesh: Mesh,
    draw: Draw,
    positions: Buffer,
}

impl CameraHelper {
    pub fn new(
        device: &Device,
        view_proj: &[f32; 16],
        options: CameraHelperOptions<'_>,
    ) -> BelfastResult<Self> {
        let positions = frustum_positions(view_proj)?;
        let colors = frustum_colors(options.color);
        let position_buffer = Buffer::from_data(
            device,
            &positions,
            BufferUsage::vertex(),
            "CameraHelperPositions",
        );
        let color_buffer =
            Buffer::from_data(device, &colors, BufferUsage::vertex(), "CameraHelperColors");
        let mesh = Mesh::new(VERTEX_COUNT)?
            .add_vertex_buffer(VertexBufferBinding {
                buffer: position_buffer.clone(),
                array_stride: 12,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                }],
                slot: Some(0),
                step_mode: None,
            })?
            .add_vertex_buffer(VertexBufferBinding {
                buffer: color_buffer,
                array_stride: 12,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                }],
                slot: Some(1),
                step_mode: None,
            })?;

        let mut draw_options = DrawOptions::new(options.label, options.format);
        draw_options.layout = Some(options.layout);
        draw_options.primitive.topology = wgpu::PrimitiveTopology::LineList;
        draw_options.depth_stencil = options.depth_stencil;
        let draw = Draw::new(
            device,
            include_str!("axis_helper.wgsl"),
            &mesh,
            draw_options,
        );

        Ok(Self {
            mesh,
            draw,
            positions: position_buffer,
        })
    }

    pub fn set_matrix(&self, view_proj: &[f32; 16]) -> BelfastResult<()> {
        let positions = frustum_positions(view_proj)?;
        self.positions.write(self.positions.device(), &positions, 0);
        Ok(())
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        camera_bind_group.bind(pass, 0);
        self.draw.draw(pass, &self.mesh, 1);
    }

    pub fn device(&self) -> &Device {
        self.draw.device()
    }

    pub fn get_bind_group_layout(&self, index: u32) -> wgpu::BindGroupLayout {
        self.draw.get_bind_group_layout(index)
    }
}

fn frustum_corners(view_proj: &[f32; 16]) -> BelfastResult<[[f32; 3]; 8]> {
    if !view_proj.iter().copied().all(f32::is_finite) {
        return Err(BelfastError::InvalidCameraMatrix);
    }
    let inverse = Mat4::from_cols_array(view_proj).inverse();
    if !inverse.is_finite() {
        return Err(BelfastError::InvalidCameraMatrix);
    }

    let mut corners = [[0.0; 3]; 8];
    for (index, clip) in CLIP_CORNERS.iter().enumerate() {
        let world = inverse * Vec4::new(clip[0], clip[1], clip[2], 1.0);
        if !world.is_finite() || world.w.abs() <= f32::EPSILON {
            return Err(BelfastError::InvalidCameraMatrix);
        }
        let point = world.truncate() / world.w;
        if !point.is_finite() {
            return Err(BelfastError::InvalidCameraMatrix);
        }
        corners[index] = point.to_array();
    }
    Ok(corners)
}

fn frustum_positions(view_proj: &[f32; 16]) -> BelfastResult<[f32; POSITION_FLOATS]> {
    let corners = frustum_corners(view_proj)?;
    let mut positions = [0.0; POSITION_FLOATS];
    for (index, [start, end]) in EDGES.iter().enumerate() {
        let offset = index * 6;
        positions[offset..offset + 3].copy_from_slice(&corners[*start]);
        positions[offset + 3..offset + 6].copy_from_slice(&corners[*end]);
    }
    Ok(positions)
}

fn frustum_colors(color: [f32; 3]) -> [f32; POSITION_FLOATS] {
    let mut colors = [0.0; POSITION_FLOATS];
    for vertex in 0..VERTEX_COUNT as usize {
        colors[vertex * 3..vertex * 3 + 3].copy_from_slice(&color);
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::{frustum_colors, frustum_corners, frustum_positions, CLIP_CORNERS, VERTEX_COUNT};
    use crate::{BelfastError, OrthographicCamera};

    fn assert_contains_point(points: &[[f32; 3]], expected: [f32; 3]) {
        let found = points.iter().any(|point| {
            (point[0] - expected[0]).abs() < 1e-4
                && (point[1] - expected[1]).abs() < 1e-4
                && (point[2] - expected[2]).abs() < 1e-4
        });
        assert!(found, "missing corner {expected:?} in {points:?}");
    }

    #[test]
    fn ortho_frustum_matches_webgpu_ndc() {
        let camera = OrthographicCamera::new(-1.0, 1.0, -1.0, 1.0, 1.0, 3.0);
        let corners = frustum_corners(&camera.view_projection_matrix()).unwrap();

        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                assert_contains_point(&corners, [x, y, 0.0]);
                assert_contains_point(&corners, [x, y, -2.0]);
            }
        }
    }

    #[test]
    fn line_list_has_twelve_edges_in_one_color() {
        let camera = OrthographicCamera::new(-1.0, 1.0, -1.0, 1.0, 1.0, 3.0);
        let positions = frustum_positions(&camera.view_projection_matrix()).unwrap();
        let colors = frustum_colors([0.2, 0.4, 0.8]);

        assert_eq!(positions.len(), VERTEX_COUNT as usize * 3);
        assert_eq!(colors.len(), positions.len());
        assert_eq!(&colors[0..3], &[0.2, 0.4, 0.8]);
        assert_eq!(&colors[69..72], &[0.2, 0.4, 0.8]);
    }

    #[test]
    fn clip_corners_use_zero_to_one_depth() {
        assert_eq!(CLIP_CORNERS[0][2], 0.0);
        assert_eq!(CLIP_CORNERS[7][2], 1.0);
    }

    #[test]
    fn rejects_zero_and_non_finite_matrices() {
        assert!(matches!(
            frustum_corners(&[0.0; 16]),
            Err(BelfastError::InvalidCameraMatrix)
        ));
        let mut nan = [0.0; 16];
        nan[0] = f32::NAN;
        assert!(matches!(
            frustum_corners(&nan),
            Err(BelfastError::InvalidCameraMatrix)
        ));
        let mut inf = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        inf[0] = f32::INFINITY;
        assert!(matches!(
            frustum_corners(&inf),
            Err(BelfastError::InvalidCameraMatrix)
        ));
    }
}
