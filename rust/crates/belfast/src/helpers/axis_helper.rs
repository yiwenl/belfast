use crate::{
    BelfastError, BelfastResult, BindGroup, Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh,
    VertexAttributeDescriptor, VertexBufferBinding,
};

pub struct AxisHelperOptions<'a> {
    pub label: &'a str,
    pub length: f32,
    pub format: wgpu::TextureFormat,
    pub layout: &'a wgpu::PipelineLayout,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> AxisHelperOptions<'a> {
    pub fn new(format: wgpu::TextureFormat, layout: &'a wgpu::PipelineLayout) -> Self {
        Self {
            label: "AxisHelper",
            length: 1.0,
            format,
            layout,
            depth_stencil: None,
        }
    }
}

pub struct AxisHelper {
    mesh: Mesh,
    draw: Draw,
}

impl AxisHelper {
    pub fn new(device: &Device, options: AxisHelperOptions<'_>) -> BelfastResult<Self> {
        if !options.length.is_finite() || options.length <= 0.0 {
            return Err(BelfastError::InvalidAxisLength);
        }

        let (positions, colors) = axis_geometry(options.length);
        let position_buffer = Buffer::from_data(
            device,
            &positions,
            BufferUsage::vertex(),
            "AxisHelperPositions",
        );
        let color_buffer =
            Buffer::from_data(device, &colors, BufferUsage::vertex(), "AxisHelperColors");
        let mesh = Mesh::new(6)?
            .add_vertex_buffer(VertexBufferBinding {
                buffer: position_buffer,
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

        Ok(Self { mesh, draw })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        camera_bind_group.bind(pass, 0);
        self.draw.draw(pass, &self.mesh, 1);
    }
}

fn axis_geometry(length: f32) -> ([f32; 18], [f32; 18]) {
    (
        [
            0.0, 0.0, 0.0, length, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, length, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, length,
        ],
        [
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            1.0,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::axis_geometry;

    #[test]
    fn geometry_contains_three_colored_positive_axes() {
        let (positions, colors) = axis_geometry(2.0);
        assert_eq!(&positions[3..6], &[2.0, 0.0, 0.0]);
        assert_eq!(&positions[9..12], &[0.0, 2.0, 0.0]);
        assert_eq!(&positions[15..18], &[0.0, 0.0, 2.0]);
        assert_eq!(&colors[0..6], &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(&colors[12..18], &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    }
}
