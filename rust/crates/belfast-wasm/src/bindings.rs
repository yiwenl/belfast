use std::collections::HashSet;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VertexBufferDescriptorInput {
    pub array_stride: u64,
    pub attributes: Vec<VertexAttributeInput>,
    #[serde(default)]
    pub slot: Option<u32>,
    #[serde(default)]
    pub step_mode: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VertexAttributeInput {
    pub shader_location: u32,
    pub format: String,
    #[serde(default)]
    pub offset: u64,
}

#[derive(Debug)]
pub(crate) struct ConvertedVertexBinding {
    pub array_stride: u64,
    pub attributes: Vec<belfast::VertexAttributeDescriptor>,
    pub slot: Option<u32>,
    pub step_mode: Option<wgpu::VertexStepMode>,
}

impl VertexBufferDescriptorInput {
    pub(crate) fn try_into_binding(self) -> Result<ConvertedVertexBinding, String> {
        if self.array_stride == 0 {
            return Err("vertex buffer arrayStride must be greater than 0".into());
        }
        if self.attributes.is_empty() {
            return Err("vertex buffer must include at least one attribute".into());
        }

        let step_mode = self.step_mode.map(parse_vertex_step_mode).transpose()?;
        let mut shader_locations = HashSet::new();
        let mut attributes = Vec::with_capacity(self.attributes.len());

        for attribute in self.attributes {
            if !shader_locations.insert(attribute.shader_location) {
                return Err(format!(
                    "duplicate vertex attribute shaderLocation {}",
                    attribute.shader_location
                ));
            }

            let (format, format_size) = parse_vertex_format(&attribute.format)?;
            let exceeds_stride = attribute
                .offset
                .checked_add(format_size)
                .is_none_or(|end| end > self.array_stride);
            if exceeds_stride {
                return Err(format!(
                    "vertex attribute at shaderLocation {} exceeds arrayStride {}",
                    attribute.shader_location, self.array_stride
                ));
            }

            attributes.push(belfast::VertexAttributeDescriptor {
                shader_location: attribute.shader_location,
                format,
                offset: attribute.offset,
            });
        }

        Ok(ConvertedVertexBinding {
            array_stride: self.array_stride,
            attributes,
            slot: self.slot,
            step_mode,
        })
    }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrimitiveStateInput {
    #[serde(default)]
    pub cull_mode: Option<String>,
}

impl PrimitiveStateInput {
    pub(crate) fn apply(self, primitive: &mut wgpu::PrimitiveState) -> Result<(), String> {
        if let Some(cull_mode) = self.cull_mode.as_deref() {
            primitive.cull_mode = parse_cull_mode(cull_mode)?;
        }
        Ok(())
    }
}

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

pub(crate) fn default_depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

pub(crate) fn parse_buffer_usage(usage: &str) -> Result<wgpu::BufferUsages, String> {
    match usage {
        "vertex" => Ok(belfast::BufferUsage::vertex()),
        "uniform" => Ok(belfast::BufferUsage::uniform()),
        "storage" => Ok(belfast::BufferUsage::storage()),
        "vertexStorage" => Ok(belfast::BufferUsage::vertex_storage()),
        _ => Err(format!("unsupported buffer usage \"{usage}\"")),
    }
}

pub(crate) fn parse_index_format(format: &str) -> Result<belfast::MeshIndexFormat, String> {
    match format {
        "uint16" => Ok(belfast::MeshIndexFormat::Uint16),
        "uint32" => Ok(belfast::MeshIndexFormat::Uint32),
        _ => Err(format!("unsupported index format \"{format}\"")),
    }
}

pub(crate) fn parse_cull_mode(cull_mode: &str) -> Result<Option<wgpu::Face>, String> {
    match cull_mode {
        "none" => Ok(None),
        "front" => Ok(Some(wgpu::Face::Front)),
        "back" => Ok(Some(wgpu::Face::Back)),
        _ => Err(format!("unsupported cull mode \"{cull_mode}\"")),
    }
}

#[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
pub(crate) fn parse_instance_count(value: Option<f64>) -> Result<u32, String> {
    let Some(value) = value else {
        return Ok(1);
    };
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 || value > f64::from(u32::MAX) {
        return Err("instanceCount must be a finite integer between 0 and 4294967295".into());
    }
    Ok(value as u32)
}

fn parse_vertex_format(format: &str) -> Result<(wgpu::VertexFormat, u64), String> {
    match format {
        "vec2" | "float32x2" => Ok((wgpu::VertexFormat::Float32x2, 8)),
        "vec3" | "float32x3" => Ok((wgpu::VertexFormat::Float32x3, 12)),
        "vec4" | "float32x4" => Ok((wgpu::VertexFormat::Float32x4, 16)),
        _ => Err(format!("unsupported vertex format \"{format}\"")),
    }
}

fn parse_vertex_step_mode(step_mode: String) -> Result<wgpu::VertexStepMode, String> {
    match step_mode.as_str() {
        "vertex" => Ok(wgpu::VertexStepMode::Vertex),
        "instance" => Ok(wgpu::VertexStepMode::Instance),
        _ => Err(format!("unsupported vertex step mode \"{step_mode}\"")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vertex_buffer_usage() {
        assert_eq!(
            parse_buffer_usage("vertex").unwrap(),
            belfast::BufferUsage::vertex()
        );
    }

    #[test]
    fn parses_uniform_buffer_usage() {
        assert_eq!(
            parse_buffer_usage("uniform").unwrap(),
            belfast::BufferUsage::uniform()
        );
    }

    #[test]
    fn parses_storage_buffer_usage() {
        assert_eq!(
            parse_buffer_usage("storage").unwrap(),
            belfast::BufferUsage::storage()
        );
        assert_eq!(
            parse_buffer_usage("vertexStorage").unwrap(),
            belfast::BufferUsage::vertex_storage()
        );
    }

    #[test]
    fn rejects_unknown_buffer_usage() {
        assert_eq!(
            parse_buffer_usage("index").unwrap_err(),
            "unsupported buffer usage \"index\""
        );
    }

    #[test]
    fn parses_index_formats() {
        assert_eq!(
            parse_index_format("uint16").unwrap(),
            belfast::MeshIndexFormat::Uint16
        );
        assert_eq!(
            parse_index_format("uint32").unwrap(),
            belfast::MeshIndexFormat::Uint32
        );
        assert_eq!(
            parse_index_format("uint8").unwrap_err(),
            "unsupported index format \"uint8\""
        );
    }

    #[test]
    fn parses_cull_modes() {
        assert_eq!(parse_cull_mode("none").unwrap(), None);
        assert_eq!(parse_cull_mode("back").unwrap(), Some(wgpu::Face::Back));
        assert_eq!(
            parse_cull_mode("sideways").unwrap_err(),
            "unsupported cull mode \"sideways\""
        );
    }

    #[test]
    fn parses_instance_count() {
        assert_eq!(parse_instance_count(None).unwrap(), 1);
        assert_eq!(parse_instance_count(Some(0.0)).unwrap(), 0);
        assert_eq!(parse_instance_count(Some(512.0)).unwrap(), 512);
        assert_eq!(
            parse_instance_count(Some(1.5)).unwrap_err(),
            "instanceCount must be a finite integer between 0 and 4294967295"
        );
    }

    #[test]
    fn builds_fixed_depth_state() {
        let state = default_depth_state();
        assert_eq!(state.format, DEPTH_FORMAT);
        assert_eq!(state.depth_write_enabled, Some(true));
        assert_eq!(state.depth_compare, Some(wgpu::CompareFunction::Less));
    }

    #[test]
    fn converts_separate_position_layout() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 8,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "vec2".into(),
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        };

        let converted = descriptor.try_into_binding().unwrap();
        assert_eq!(converted.array_stride, 8);
        assert_eq!(
            converted.attributes[0].format,
            wgpu::VertexFormat::Float32x2
        );
        assert_eq!(converted.slot, Some(0));
    }

    #[test]
    fn parses_gl_matrix_formats_and_webgpu_aliases() {
        assert_eq!(
            parse_vertex_format("vec2").unwrap(),
            (wgpu::VertexFormat::Float32x2, 8)
        );
        assert_eq!(
            parse_vertex_format("vec3").unwrap(),
            (wgpu::VertexFormat::Float32x3, 12)
        );
        assert_eq!(
            parse_vertex_format("vec4").unwrap(),
            (wgpu::VertexFormat::Float32x4, 16)
        );
        assert_eq!(
            parse_vertex_format("float32x4").unwrap(),
            (wgpu::VertexFormat::Float32x4, 16)
        );
    }

    #[test]
    fn rejects_attribute_past_array_stride() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 8,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "vec3".into(),
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "vertex attribute at shaderLocation 0 exceeds arrayStride 8"
        );
    }

    #[test]
    fn rejects_zero_array_stride() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 0,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "vec2".into(),
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "vertex buffer arrayStride must be greater than 0"
        );
    }

    #[test]
    fn rejects_empty_attributes() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 8,
            attributes: vec![],
            slot: None,
            step_mode: None,
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "vertex buffer must include at least one attribute"
        );
    }

    #[test]
    fn rejects_unknown_vertex_format() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 4,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "float16x2".into(),
                offset: 0,
            }],
            slot: None,
            step_mode: None,
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "unsupported vertex format \"float16x2\""
        );
    }

    #[test]
    fn rejects_duplicate_shader_locations() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 16,
            attributes: vec![
                VertexAttributeInput {
                    shader_location: 0,
                    format: "vec2".into(),
                    offset: 0,
                },
                VertexAttributeInput {
                    shader_location: 0,
                    format: "vec2".into(),
                    offset: 8,
                },
            ],
            slot: None,
            step_mode: None,
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "duplicate vertex attribute shaderLocation 0"
        );
    }

    #[test]
    fn rejects_unknown_step_mode() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 8,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "vec2".into(),
                offset: 0,
            }],
            slot: None,
            step_mode: Some("draw".into()),
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "unsupported vertex step mode \"draw\""
        );
    }
}
