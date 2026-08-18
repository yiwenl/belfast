use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice, WasmMesh};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DrawOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

fn parse_and_validate_shader(shader_code: &str) -> Result<naga::Module, String> {
    let module = naga::front::wgsl::parse_str(shader_code)
        .map_err(|error| format!("WGSL parse failed: {error}"))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map_err(|error| format!("WGSL validation failed: {error}"))?;

    Ok(module)
}

#[cfg(test)]
fn validate_shader_code(shader_code: &str) -> Result<(), String> {
    parse_and_validate_shader(shader_code).map(|_| ())
}

#[cfg(test)]
fn validate_draw_interface(
    shader_code: &str,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
) -> Result<(), String> {
    let module = parse_and_validate_shader(shader_code)?;
    validate_module_draw_interface(&module, mesh_attributes)
}

fn validate_module_draw_interface(
    module: &naga::Module,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
) -> Result<(), String> {
    let vertex_entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == "vs_main" && entry.stage == naga::ShaderStage::Vertex)
        .ok_or_else(|| "shader is missing required vertex entry point \"vs_main\"".to_owned())?;
    module
        .entry_points
        .iter()
        .find(|entry| entry.name == "fs_main" && entry.stage == naga::ShaderStage::Fragment)
        .ok_or_else(|| "shader is missing required fragment entry point \"fs_main\"".to_owned())?;

    let required_attributes = required_vertex_attributes(module, vertex_entry)?;
    for (location, expected_format) in required_attributes {
        let Some((_, actual_format)) = mesh_attributes
            .iter()
            .find(|(mesh_location, _)| *mesh_location == location)
        else {
            return Err(format!(
                "mesh is missing shader vertex input @location({location}) ({expected_format:?})"
            ));
        };
        if *actual_format != expected_format {
            return Err(format!(
                "mesh vertex input @location({location}) has format {actual_format:?}, expected {expected_format:?}"
            ));
        }
    }

    Ok(())
}

fn required_vertex_attributes(
    module: &naga::Module,
    vertex_entry: &naga::EntryPoint,
) -> Result<Vec<(u32, wgpu::VertexFormat)>, String> {
    let mut attributes = Vec::new();

    for argument in &vertex_entry.function.arguments {
        if let Some(binding) = argument.binding.as_ref() {
            collect_bound_vertex_input(module, argument.ty, binding, &mut attributes)?;
            continue;
        }

        let naga::TypeInner::Struct { members, .. } = &module.types[argument.ty].inner else {
            let name = argument.name.as_deref().unwrap_or("<unnamed>");
            return Err(format!(
                "unsupported shader vertex input {name:?}; expected @location, @builtin, or an input struct"
            ));
        };
        for member in members {
            let Some(binding) = member.binding.as_ref() else {
                let name = member.name.as_deref().unwrap_or("<unnamed>");
                return Err(format!(
                    "unsupported shader vertex input struct member {name:?} without @location or @builtin"
                ));
            };
            collect_bound_vertex_input(module, member.ty, binding, &mut attributes)?;
        }
    }

    Ok(attributes)
}

fn collect_bound_vertex_input(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: &naga::Binding,
    attributes: &mut Vec<(u32, wgpu::VertexFormat)>,
) -> Result<(), String> {
    let naga::Binding::Location { location, .. } = binding else {
        return Ok(());
    };
    let format = match &module.types[ty].inner {
        naga::TypeInner::Vector {
            size: naga::VectorSize::Bi,
            scalar:
                naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                },
        } => wgpu::VertexFormat::Float32x2,
        naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            scalar:
                naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                },
        } => wgpu::VertexFormat::Float32x3,
        _ => {
            return Err(format!(
                "unsupported shader vertex input @location({location}) type; expected vec2<f32> or vec3<f32>"
            ));
        }
    };
    attributes.push((*location, format));

    Ok(())
}

#[wasm_bindgen(js_name = Draw)]
pub struct WasmDraw {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) inner: belfast::Draw,
}

#[wasm_bindgen(js_class = Draw)]
impl WasmDraw {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        shader_code: &str,
        mesh: &WasmMesh,
        options: JsValue,
    ) -> Result<WasmDraw, JsValue> {
        let options: DrawOptionsInput =
            serde_wasm_bindgen::from_value(options).map_err(to_js_error)?;
        let module = parse_and_validate_shader(shader_code).map_err(to_js_error)?;
        let vertex_layouts = mesh.inner().vertex_layouts();
        let mesh_attributes: Vec<_> = vertex_layouts
            .iter()
            .flatten()
            .flat_map(|layout| layout.attributes)
            .map(|attribute| (attribute.shader_location, attribute.format))
            .collect();
        validate_module_draw_interface(&module, &mesh_attributes).map_err(to_js_error)?;
        let inner = belfast::Draw::new(
            &device.inner,
            shader_code,
            mesh.inner(),
            belfast::DrawOptions::new(
                options.label.as_deref().unwrap_or("Draw"),
                device.inner.format(),
            ),
        );

        Ok(Self { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORED_TRIANGLE_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn vs_main(
    @location(0) position: vec2f,
    @location(1) color: vec3f,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4f(position, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return vec4f(input.color, 1.0);
}
"#;

    const STRUCT_INPUT_SHADER: &str = r#"
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec2f,
    @location(1) color: vec3f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4f(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return vec4f(input.color, 1.0);
}
"#;

    const TRIANGLE_ATTRIBUTES: &[(u32, wgpu::VertexFormat)] = &[
        (0, wgpu::VertexFormat::Float32x2),
        (1, wgpu::VertexFormat::Float32x3),
    ];

    #[test]
    fn accepts_valid_colored_triangle_shader() {
        assert_eq!(
            validate_draw_interface(COLORED_TRIANGLE_SHADER, TRIANGLE_ATTRIBUTES),
            Ok(())
        );
    }

    #[test]
    fn rejects_malformed_wgsl() {
        let error = validate_shader_code("@vertex fn vs_main(").unwrap_err();

        assert!(error.starts_with("WGSL parse failed:"), "{error}");
    }

    #[test]
    fn rejects_semantically_invalid_wgsl() {
        let error = validate_shader_code("@vertex fn vs_main() {}").unwrap_err();

        assert!(error.starts_with("WGSL validation failed:"), "{error}");
    }

    #[test]
    fn rejects_missing_vertex_entry_point() {
        let shader = r#"
@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[]),
            Err("shader is missing required vertex entry point \"vs_main\"".into())
        );
    }

    #[test]
    fn rejects_missing_fragment_entry_point() {
        let shader = r#"
@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[]),
            Err("shader is missing required fragment entry point \"fs_main\"".into())
        );
    }

    #[test]
    fn rejects_missing_mesh_location() {
        assert_eq!(
            validate_draw_interface(
                COLORED_TRIANGLE_SHADER,
                &[(0, wgpu::VertexFormat::Float32x2)]
            ),
            Err("mesh is missing shader vertex input @location(1) (Float32x3)".into())
        );
    }

    #[test]
    fn rejects_mismatched_mesh_format() {
        assert_eq!(
            validate_draw_interface(
                COLORED_TRIANGLE_SHADER,
                &[
                    (0, wgpu::VertexFormat::Float32x3),
                    (1, wgpu::VertexFormat::Float32x3),
                ]
            ),
            Err("mesh vertex input @location(0) has format Float32x3, expected Float32x2".into())
        );
    }

    #[test]
    fn accepts_struct_inputs_builtins_and_extra_mesh_attributes() {
        let attributes = [
            (0, wgpu::VertexFormat::Float32x2),
            (1, wgpu::VertexFormat::Float32x3),
            (2, wgpu::VertexFormat::Float32x2),
        ];

        assert_eq!(
            validate_draw_interface(STRUCT_INPUT_SHADER, &attributes),
            Ok(())
        );
    }

    #[test]
    fn rejects_unsupported_vertex_input_type() {
        let shader = r#"
@vertex
fn vs_main(@location(0) position: vec4f) -> @builtin(position) vec4f {
    return position;
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[(0, wgpu::VertexFormat::Float32x2)]),
            Err(
                "unsupported shader vertex input @location(0) type; expected vec2<f32> or vec3<f32>"
                    .into()
            )
        );
    }
}
