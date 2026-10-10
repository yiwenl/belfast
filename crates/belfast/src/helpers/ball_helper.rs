use crate::{
    BelfastError, BelfastResult, BindGroup, Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh,
    MeshIndexFormat, VertexAttributeDescriptor, VertexBufferBinding,
};

const SEGMENTS: usize = 16;

pub struct BallHelperOptions<'a> {
    pub label: &'a str,
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub format: wgpu::TextureFormat,
    pub layout: &'a wgpu::PipelineLayout,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> BallHelperOptions<'a> {
    pub fn new(format: wgpu::TextureFormat, layout: &'a wgpu::PipelineLayout) -> Self {
        Self {
            label: "BallHelper",
            position: [0.0, 0.0, 0.0],
            radius: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            format,
            layout,
            depth_stencil: None,
        }
    }
}

pub struct BallHelper {
    mesh: Mesh,
    draw: Draw,
    positions: Buffer,
    colors: Buffer,
}

impl BallHelper {
    pub fn new(device: &Device, options: BallHelperOptions<'_>) -> BelfastResult<Self> {
        validate_ball(options.position, options.radius, options.color)?;

        let (positions, colors, indices) =
            ball_geometry(options.position, options.radius, options.color);
        let position_buffer = Buffer::from_data(
            device,
            &positions,
            BufferUsage::vertex(),
            "BallHelperPositions",
        );
        let color_buffer =
            Buffer::from_data(device, &colors, BufferUsage::vertex(), "BallHelperColors");
        let index_buffer =
            Buffer::from_data(device, &indices, BufferUsage::index(), "BallHelperIndices");
        let mut mesh = Mesh::new(positions.len() as u32 / 3)?
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
                buffer: color_buffer.clone(),
                array_stride: 16,
                attributes: vec![VertexAttributeDescriptor {
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                }],
                slot: Some(1),
                step_mode: None,
            })?;
        mesh.set_index_buffer(index_buffer, indices.len() as u32, MeshIndexFormat::Uint16)?;

        let mut draw_options = DrawOptions::new(options.label, options.format);
        draw_options.layout = Some(options.layout);
        draw_options.primitive.cull_mode = Some(wgpu::Face::Back);
        draw_options.depth_stencil = options.depth_stencil;
        draw_options.targets = vec![Some(wgpu::ColorTargetState {
            format: options.format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let draw = Draw::new(
            device,
            include_str!("ball_helper.wgsl"),
            &mesh,
            draw_options,
        );

        Ok(Self {
            mesh,
            draw,
            positions: position_buffer,
            colors: color_buffer,
        })
    }

    pub fn set(&self, position: [f32; 3], radius: f32, color: [f32; 4]) -> BelfastResult<()> {
        validate_ball(position, radius, color)?;
        let (positions, colors, _) = ball_geometry(position, radius, color);
        self.positions.write(self.positions.device(), &positions, 0);
        self.colors.write(self.colors.device(), &colors, 0);
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

fn validate_ball(position: [f32; 3], radius: f32, color: [f32; 4]) -> BelfastResult<()> {
    if position.iter().copied().all(f32::is_finite)
        && color.iter().copied().all(f32::is_finite)
        && radius.is_finite()
        && radius > 0.0
    {
        Ok(())
    } else {
        Err(BelfastError::InvalidBallParams)
    }
}

fn ball_geometry(
    position: [f32; 3],
    radius: f32,
    color: [f32; 4],
) -> (Vec<f32>, Vec<f32>, Vec<u16>) {
    let stacks = SEGMENTS;
    let slices = SEGMENTS;
    let vertex_count = (stacks + 1) * (slices + 1);
    let mut positions = Vec::with_capacity(vertex_count * 3);
    let mut colors = Vec::with_capacity(vertex_count * 4);

    for stack in 0..=stacks {
        let phi = stack as f32 / stacks as f32 * std::f32::consts::PI;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for slice in 0..=slices {
            let theta = slice as f32 / slices as f32 * std::f32::consts::TAU;
            let (sin_theta, cos_theta) = theta.sin_cos();
            positions.extend_from_slice(&[
                position[0] + sin_phi * cos_theta * radius,
                position[1] + cos_phi * radius,
                position[2] + sin_phi * sin_theta * radius,
            ]);
            colors.extend_from_slice(&color);
        }
    }

    let mut indices = Vec::with_capacity(stacks * slices * 6);
    let row = (slices + 1) as u16;
    for stack in 0..stacks {
        for slice in 0..slices {
            let i0 = stack as u16 * row + slice as u16;
            let i1 = i0 + 1;
            let i2 = i0 + row;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i1, i2, i1, i3, i2]);
        }
    }

    (positions, colors, indices)
}

#[cfg(test)]
mod tests {
    use super::{ball_geometry, validate_ball, SEGMENTS};

    #[test]
    fn sphere_poles_follow_position_and_radius() {
        let (positions, colors, indices) =
            ball_geometry([1.0, 2.0, 3.0], 2.0, [0.1, 0.2, 0.3, 0.4]);
        let vertex_count = (SEGMENTS + 1) * (SEGMENTS + 1);

        assert_eq!(positions.len(), vertex_count * 3);
        assert_eq!(colors.len(), vertex_count * 4);
        assert_eq!(indices.len(), SEGMENTS * SEGMENTS * 6);
        assert_eq!(&positions[0..3], &[1.0, 4.0, 3.0]);
        let south = SEGMENTS * (SEGMENTS + 1) * 3;
        let south_vertex = &positions[south..south + 3];
        assert!((south_vertex[0] - 1.0).abs() < 1e-5);
        assert!((south_vertex[1] - 0.0).abs() < 1e-5);
        assert!((south_vertex[2] - 3.0).abs() < 1e-5);
        assert_eq!(&colors[0..4], &[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(&colors[colors.len() - 4..], &[0.1, 0.2, 0.3, 0.4]);

        for vertex in positions.chunks(3) {
            let distance =
                ((vertex[0] - 1.0).powi(2) + (vertex[1] - 2.0).powi(2) + (vertex[2] - 3.0).powi(2))
                    .sqrt();
            assert!((distance - 2.0).abs() < 1e-4, "{distance}");
        }
    }

    #[test]
    fn rejects_invalid_position_radius_and_color() {
        assert!(validate_ball([0.0, 0.0, 0.0], 1.0, [1.0, 1.0, 1.0, 1.0]).is_ok());
        for radius in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                validate_ball([0.0, 0.0, 0.0], radius, [1.0, 1.0, 1.0, 1.0]).is_err(),
                "accepted radius {radius:?}"
            );
        }
        assert!(validate_ball([f32::NAN, 0.0, 0.0], 1.0, [1.0, 1.0, 1.0, 1.0]).is_err());
        assert!(validate_ball([0.0, 0.0, 0.0], 1.0, [1.0, f32::INFINITY, 1.0, 1.0]).is_err());
    }
}
