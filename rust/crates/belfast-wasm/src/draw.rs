use std::collections::BTreeMap;

use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice, WasmMesh};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DrawOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

#[cfg(test)]
fn parse_and_validate_shader(shader_code: &str) -> Result<naga::Module, String> {
    let module = naga::front::wgsl::parse_str(shader_code)
        .map_err(|error| format!("WGSL parse failed: {error}"))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
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
    validate_draw_interface_with_limit(shader_code, mesh_attributes, 16)
}

#[cfg(test)]
fn validate_draw_interface_with_limit(
    shader_code: &str,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
    max_inter_stage_shader_variables: u32,
) -> Result<(), String> {
    parse_and_validate_draw_shader(
        shader_code,
        mesh_attributes,
        max_inter_stage_shader_variables,
    )
    .map(|_| ())
}

fn parse_and_validate_draw_shader(
    shader_code: &str,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
    max_inter_stage_shader_variables: u32,
) -> Result<naga::Module, String> {
    let module = naga::front::wgsl::parse_str(shader_code)
        .map_err(|error| format!("WGSL parse failed: {error}"))?;
    validate_module_draw_interface(&module, mesh_attributes, max_inter_stage_shader_variables)?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|error| format!("WGSL validation failed: {error}"))?;
    Ok(module)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InterfaceType {
    F32,
    Vec2F32,
    Vec3F32,
    Vec4F32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InterfaceField {
    ty: InterfaceType,
    interpolation: Option<naga::Interpolation>,
    sampling: Option<naga::Sampling>,
}

impl std::fmt::Display for InterfaceType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::F32 => "f32",
            Self::Vec2F32 => "vec2<f32>",
            Self::Vec3F32 => "vec3<f32>",
            Self::Vec4F32 => "vec4<f32>",
        })
    }
}

fn validate_module_draw_interface(
    module: &naga::Module,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
    max_inter_stage_shader_variables: u32,
) -> Result<(), String> {
    if let Some(binding) = module
        .global_variables
        .iter()
        .find_map(|(_, variable)| variable.binding.as_ref())
    {
        return Err(format!(
            "shader resource @group({}) @binding({}) is unsupported",
            binding.group, binding.binding
        ));
    }

    let vertex_entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == "vs_main" && entry.stage == naga::ShaderStage::Vertex)
        .ok_or_else(|| "shader is missing required vertex entry point \"vs_main\"".to_owned())?;
    let fragment_entry = module
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

    let vertex_outputs = validate_vertex_outputs(module, vertex_entry)?;
    validate_interstage_limits(&vertex_outputs, max_inter_stage_shader_variables)?;
    validate_fragment_inputs(module, fragment_entry, &vertex_outputs)?;
    validate_fragment_output(module, fragment_entry)?;

    Ok(())
}

fn validate_interstage_limits(
    vertex_outputs: &BTreeMap<u32, InterfaceField>,
    max_inter_stage_shader_variables: u32,
) -> Result<(), String> {
    if vertex_outputs.len() > max_inter_stage_shader_variables as usize {
        return Err(format!(
            "shader uses {} inter-stage variables, but the device supports at most {max_inter_stage_shader_variables}",
            vertex_outputs.len()
        ));
    }

    if let Some(location) = vertex_outputs
        .keys()
        .find(|location| **location >= max_inter_stage_shader_variables)
    {
        return Err(format!(
            "shader inter-stage @location({location}) exceeds the device limit of {max_inter_stage_shader_variables} variables"
        ));
    }

    Ok(())
}

fn validate_vertex_outputs(
    module: &naga::Module,
    entry: &naga::EntryPoint,
) -> Result<BTreeMap<u32, InterfaceField>, String> {
    let result = entry
        .function
        .result
        .as_ref()
        .ok_or_else(|| "vertex shader must output @builtin(position) vec4<f32>".to_owned())?;
    let bindings =
        bound_interface_fields(module, result.ty, result.binding.as_ref(), "vertex output")?;
    let mut has_position = false;
    let mut locations = BTreeMap::new();

    for (binding, ty) in bindings {
        match binding {
            naga::Binding::BuiltIn(naga::BuiltIn::Position { .. }) => {
                if interface_type(module, ty) != Some(InterfaceType::Vec4F32) {
                    return Err("vertex shader must output @builtin(position) vec4<f32>".into());
                }
                has_position = true;
            }
            naga::Binding::Location {
                location,
                interpolation,
                sampling,
                ..
            } => {
                let ty = interface_type(module, ty).ok_or_else(|| {
                    format!("unsupported vertex output @location({location}) type")
                })?;
                locations.insert(
                    location,
                    InterfaceField {
                        ty,
                        interpolation,
                        sampling,
                    },
                );
            }
            naga::Binding::BuiltIn(builtin) => {
                return Err(format!("unsupported vertex output builtin {builtin:?}"));
            }
        }
    }

    if !has_position {
        return Err("vertex shader must output @builtin(position) vec4<f32>".into());
    }
    Ok(locations)
}

fn validate_fragment_inputs(
    module: &naga::Module,
    entry: &naga::EntryPoint,
    vertex_outputs: &BTreeMap<u32, InterfaceField>,
) -> Result<(), String> {
    for argument in &entry.function.arguments {
        for (binding, ty) in bound_interface_fields(
            module,
            argument.ty,
            argument.binding.as_ref(),
            "fragment input",
        )? {
            let naga::Binding::Location {
                location,
                interpolation,
                sampling,
                ..
            } = binding
            else {
                continue;
            };
            let fragment_type = interface_type(module, ty)
                .ok_or_else(|| format!("unsupported fragment input @location({location}) type"))?;
            let vertex_type = vertex_outputs.get(&location).ok_or_else(|| {
                format!("fragment input @location({location}) has no matching vertex output")
            })?;
            if fragment_type != vertex_type.ty {
                return Err(format!(
                    "fragment input @location({location}) has type {fragment_type}, but vertex output has type {}",
                    vertex_type.ty
                ));
            }
            if interpolation != vertex_type.interpolation || sampling != vertex_type.sampling {
                return Err(format!(
                    "fragment input @location({location}) interpolation does not match vertex output"
                ));
            }
        }
    }
    Ok(())
}

fn validate_fragment_output(module: &naga::Module, entry: &naga::EntryPoint) -> Result<(), String> {
    let Some(result) = entry.function.result.as_ref() else {
        return Err("fragment shader must output exactly @location(0) vec4<f32>".into());
    };
    let bindings = bound_interface_fields(
        module,
        result.ty,
        result.binding.as_ref(),
        "fragment output",
    )?;
    if bindings.len() != 1 {
        return Err("fragment shader must output exactly @location(0) vec4<f32>".into());
    }
    let (binding, ty) = &bindings[0];
    let is_color_zero = matches!(
        binding,
        naga::Binding::Location {
            location: 0,
            blend_src: None,
            ..
        }
    );
    if !is_color_zero || interface_type(module, *ty) != Some(InterfaceType::Vec4F32) {
        return Err("fragment shader must output exactly @location(0) vec4<f32>".into());
    }
    Ok(())
}

fn bound_interface_fields(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    binding: Option<&naga::Binding>,
    subject: &str,
) -> Result<Vec<(naga::Binding, naga::Handle<naga::Type>)>, String> {
    if let Some(binding) = binding {
        return Ok(vec![(binding.clone(), ty)]);
    }

    let naga::TypeInner::Struct { members, .. } = &module.types[ty].inner else {
        return Err(format!(
            "unsupported {subject} without an interface binding"
        ));
    };
    members
        .iter()
        .map(|member| {
            member
                .binding
                .clone()
                .map(|binding| (binding, member.ty))
                .ok_or_else(|| format!("unsupported {subject} struct member without a binding"))
        })
        .collect()
}

fn interface_type(module: &naga::Module, ty: naga::Handle<naga::Type>) -> Option<InterfaceType> {
    match module.types[ty].inner {
        naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Float,
            width: 4,
        }) => Some(InterfaceType::F32),
        naga::TypeInner::Vector {
            size: naga::VectorSize::Bi,
            scalar:
                naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                },
        } => Some(InterfaceType::Vec2F32),
        naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            scalar:
                naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                },
        } => Some(InterfaceType::Vec3F32),
        naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            scalar:
                naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                },
        } => Some(InterfaceType::Vec4F32),
        _ => None,
    }
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
        belfast::Draw::validate_mesh_device(&device.inner, mesh.inner()).map_err(to_js_error)?;
        let vertex_layouts = mesh.inner().vertex_layouts();
        let mesh_attributes: Vec<_> = vertex_layouts
            .iter()
            .flatten()
            .flat_map(|layout| layout.attributes)
            .map(|attribute| (attribute.shader_location, attribute.format))
            .collect();
        parse_and_validate_draw_shader(
            shader_code,
            &mesh_attributes,
            device.inner.gpu().limits().max_inter_stage_shader_variables,
        )
        .map_err(to_js_error)?;
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

    #[test]
    fn rejects_bind_group_resources_until_facade_can_bind_them() {
        let shader = r#"
@group(0) @binding(2) var<uniform> tint: vec4f;

@vertex
fn vs_main(@location(0) position: vec2f) -> @builtin(position) vec4f {
    return vec4f(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return tint;
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[(0, wgpu::VertexFormat::Float32x2)]),
            Err("shader resource @group(0) @binding(2) is unsupported".into())
        );
    }

    #[test]
    fn rejects_mismatched_vertex_fragment_interface() {
        let shader = r#"
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn vs_main(@location(0) position: vec2f) -> VertexOutput {
    return VertexOutput(vec4f(position, 0.0, 1.0), vec3f(1.0));
}

@fragment
fn fs_main(@location(0) color: vec2f) -> @location(0) vec4f {
    return vec4f(color, 0.0, 1.0);
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[(0, wgpu::VertexFormat::Float32x2)]),
            Err(
                "fragment input @location(0) has type vec2<f32>, but vertex output has type vec3<f32>"
                    .into()
            )
        );
    }

    #[test]
    fn rejects_mismatched_vertex_fragment_interpolation() {
        let shader = r#"
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) @interpolate(flat) color: vec3f,
}

@vertex
fn vs_main(@location(0) position: vec2f) -> VertexOutput {
    return VertexOutput(vec4f(position, 0.0, 1.0), vec3f(1.0));
}

@fragment
fn fs_main(@location(0) color: vec3f) -> @location(0) vec4f {
    return vec4f(color, 1.0);
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[(0, wgpu::VertexFormat::Float32x2)]),
            Err("fragment input @location(0) interpolation does not match vertex output".into())
        );
    }

    #[test]
    fn rejects_interstage_location_beyond_device_limit() {
        let shader = r#"
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(4) color: vec3f,
}

@vertex
fn vs_main() -> VertexOutput {
    return VertexOutput(vec4f(0.0, 0.0, 0.0, 1.0), vec3f(1.0));
}

@fragment
fn fs_main(@location(4) color: vec3f) -> @location(0) vec4f {
    return vec4f(color, 1.0);
}
"#;

        assert_eq!(
            validate_draw_interface_with_limit(shader, &[], 4),
            Err("shader inter-stage @location(4) exceeds the device limit of 4 variables".into())
        );
    }

    #[test]
    fn rejects_vertex_shader_without_position_output() {
        let shader = r#"
@vertex
fn vs_main(@location(0) position: vec2f) -> @location(0) vec2f {
    return position;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    return vec4f(position, 0.0, 1.0);
}
"#;

        assert_eq!(
            validate_draw_interface(shader, &[(0, wgpu::VertexFormat::Float32x2)]),
            Err("vertex shader must output @builtin(position) vec4<f32>".into())
        );
    }

    #[test]
    fn rejects_unsupported_fragment_outputs() {
        let wrong_color_type = r#"
@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0);
}

@fragment
fn fs_main() -> @location(0) vec3f {
    return vec3f(1.0);
}
"#;
        assert_eq!(
            validate_draw_interface(wrong_color_type, &[]),
            Err("fragment shader must output exactly @location(0) vec4<f32>".into())
        );

        let depth_output = r#"
struct FragmentOutput {
    @location(0) color: vec4f,
    @builtin(frag_depth) depth: f32,
}

@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0);
}

@fragment
fn fs_main() -> FragmentOutput {
    return FragmentOutput(vec4f(1.0), 0.5);
}
"#;
        assert_eq!(
            validate_draw_interface(depth_output, &[]),
            Err("fragment shader must output exactly @location(0) vec4<f32>".into())
        );
    }
}
