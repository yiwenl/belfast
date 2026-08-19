use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice, WasmMesh};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DrawOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ShaderResourceLayout {
    None,
    TextureSampler {
        group: u32,
        texture_binding: u32,
        sampler_binding: u32,
    },
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
) -> Result<ShaderResourceLayout, String> {
    validate_draw_interface_with_limit(shader_code, mesh_attributes, 16)
}

#[cfg(test)]
fn validate_draw_interface_with_limit(
    shader_code: &str,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
    max_inter_stage_shader_variables: u32,
) -> Result<ShaderResourceLayout, String> {
    parse_and_validate_draw_shader(
        shader_code,
        mesh_attributes,
        max_inter_stage_shader_variables,
    )
}

fn parse_and_validate_draw_shader(
    shader_code: &str,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
    max_inter_stage_shader_variables: u32,
) -> Result<ShaderResourceLayout, String> {
    let module = naga::front::wgsl::parse_str(shader_code)
        .map_err(|error| format!("WGSL parse failed: {error}"))?;
    let resources =
        validate_module_draw_interface(&module, mesh_attributes, max_inter_stage_shader_variables)?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|error| format!("WGSL validation failed: {error}"))?;
    Ok(resources)
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
) -> Result<ShaderResourceLayout, String> {
    let resources = validate_shader_resource_layout(module)?;

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

    Ok(resources)
}

fn validate_shader_resource_layout(module: &naga::Module) -> Result<ShaderResourceLayout, String> {
    let mut texture = None;
    let mut sampler = None;

    for (_, variable) in module.global_variables.iter() {
        let Some(binding) = variable.binding else {
            continue;
        };
        if binding.group != 0 {
            return Err(unsupported_shader_resource(binding));
        }

        match &module.types[variable.ty].inner {
            naga::TypeInner::Image {
                dim: naga::ImageDimension::D2,
                arrayed: false,
                class:
                    naga::ImageClass::Sampled {
                        kind: naga::ScalarKind::Float,
                        multi: false,
                    },
            } => {
                if texture.replace(binding).is_some() {
                    return Err(unsupported_shader_resource(binding));
                }
            }
            naga::TypeInner::Sampler { comparison: false } => {
                if sampler.replace(binding).is_some() {
                    return Err(unsupported_shader_resource(binding));
                }
            }
            naga::TypeInner::Sampler { comparison: true } => {
                return Err(format!(
                    "shader resource @group({}) @binding({}) must be a filtering sampler",
                    binding.group, binding.binding
                ));
            }
            _ => return Err(unsupported_shader_resource(binding)),
        }
    }

    match (texture, sampler) {
        (None, None) => Ok(ShaderResourceLayout::None),
        (Some(texture), Some(sampler)) if texture.group == sampler.group => {
            Ok(ShaderResourceLayout::TextureSampler {
                group: texture.group,
                texture_binding: texture.binding,
                sampler_binding: sampler.binding,
            })
        }
        (Some(_), None) => {
            Err("sampled texture requires one filtering sampler in the same bind group".into())
        }
        (None, Some(_)) => {
            Err("filtering sampler requires one sampled texture in the same bind group".into())
        }
        (Some(_), Some(_)) => {
            Err("sampled texture and filtering sampler must use the same bind group".into())
        }
    }
}

fn unsupported_shader_resource(binding: naga::ResourceBinding) -> String {
    format!(
        "unsupported shader resource @group({}) @binding({})",
        binding.group, binding.binding
    )
}

fn replacement_layout_matches(
    pipeline_layout: &belfast::MeshLayoutSignature,
    replacement_layout: &belfast::MeshLayoutSignature,
) -> bool {
    pipeline_layout == replacement_layout
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
    pub(crate) state: Rc<DrawState>,
}

pub(crate) struct DrawState {
    draw: belfast::Draw,
    mesh: RefCell<belfast::Mesh>,
    #[allow(dead_code)]
    resources: ShaderResourceLayout,
}

impl DrawState {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn draw(&self) -> &belfast::Draw {
        &self.draw
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn mesh(&self) -> std::cell::Ref<'_, belfast::Mesh> {
        self.mesh.borrow()
    }
}

#[wasm_bindgen(js_class = Draw)]
impl WasmDraw {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        shader_code: &str,
        mesh: WasmMesh,
        options: JsValue,
    ) -> Result<WasmDraw, JsValue> {
        let options: DrawOptionsInput =
            serde_wasm_bindgen::from_value(options).map_err(to_js_error)?;
        let mesh = mesh.into_inner();
        belfast::Draw::validate_mesh_device(&device.inner, &mesh).map_err(to_js_error)?;
        let mesh_attributes: Vec<_> = {
            let vertex_layouts = mesh.vertex_layouts();
            vertex_layouts
                .iter()
                .flatten()
                .flat_map(|layout| layout.attributes)
                .map(|attribute| (attribute.shader_location, attribute.format))
                .collect()
        };
        let resources = parse_and_validate_draw_shader(
            shader_code,
            &mesh_attributes,
            device.inner.gpu().limits().max_inter_stage_shader_variables,
        )
        .map_err(to_js_error)?;
        let draw = belfast::Draw::new(
            &device.inner,
            shader_code,
            &mesh,
            belfast::DrawOptions::new(
                options.label.as_deref().unwrap_or("Draw"),
                device.inner.format(),
            ),
        );

        Ok(Self {
            state: Rc::new(DrawState {
                draw,
                mesh: RefCell::new(mesh),
                resources,
            }),
        })
    }

    #[wasm_bindgen(js_name = setMesh)]
    pub fn set_mesh(&self, mesh: WasmMesh) -> Result<(), JsValue> {
        let mesh = mesh.into_inner();
        self.state
            .draw
            .validate_for_render(self.state.draw.device(), &mesh)
            .map_err(to_js_error)?;
        debug_assert!(replacement_layout_matches(
            self.state.draw.mesh_layout_signature(),
            &mesh.layout_signature()
        ));
        self.state.mesh.replace(mesh);
        Ok(())
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

    const TEXTURE_SHADER: &str = r#"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

@vertex
fn vs_main(@location(0) position: vec2f) -> @builtin(position) vec4f {
    return vec4f(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return textureSample(image, image_sampler, vec2f(0.5));
}
"#;

    const STORAGE_TEXTURE_SHADER: &str = r#"
@group(0) @binding(0) var image: texture_storage_2d<rgba8unorm, write>;

@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    const COMPARISON_SAMPLER_SHADER: &str = r#"
@group(0) @binding(0) var image_sampler: sampler_comparison;

@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    const TEXTURE_ONLY_SHADER: &str = r#"
@group(0) @binding(0) var image: texture_2d<f32>;

@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return textureLoad(image, vec2i(0), 0);
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
            Ok(ShaderResourceLayout::None)
        );
    }

    #[test]
    fn accepts_supported_texture_and_sampler_pair() {
        let layout =
            validate_draw_interface_with_limit(TEXTURE_SHADER, TRIANGLE_ATTRIBUTES, 16).unwrap();
        assert_eq!(
            layout,
            ShaderResourceLayout::TextureSampler {
                group: 0,
                texture_binding: 0,
                sampler_binding: 1,
            }
        );
    }

    #[test]
    fn rejects_storage_texture_and_comparison_sampler() {
        assert!(
            validate_draw_interface_with_limit(STORAGE_TEXTURE_SHADER, &[], 16)
                .unwrap_err()
                .contains("unsupported shader resource")
        );
        assert!(
            validate_draw_interface_with_limit(COMPARISON_SAMPLER_SHADER, &[], 16)
                .unwrap_err()
                .contains("filtering sampler")
        );
    }

    #[test]
    fn rejects_incomplete_texture_sampler_pair() {
        assert_eq!(
            validate_draw_interface_with_limit(TEXTURE_ONLY_SHADER, &[], 16),
            Err("sampled texture requires one filtering sampler in the same bind group".into())
        );
    }

    #[test]
    fn replacement_mesh_layout_must_match_pipeline_signature() {
        let pipeline_mesh = mesh_with_position_format(wgpu::VertexFormat::Float32x2);
        let compatible_mesh = mesh_with_position_format(wgpu::VertexFormat::Float32x2);
        let incompatible_mesh = mesh_with_position_format(wgpu::VertexFormat::Float32x3);
        let pipeline_layout = pipeline_mesh.layout_signature();

        assert!(replacement_layout_matches(
            &pipeline_layout,
            &compatible_mesh.layout_signature()
        ));
        assert!(!replacement_layout_matches(
            &pipeline_layout,
            &incompatible_mesh.layout_signature()
        ));
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
            Ok(ShaderResourceLayout::None)
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
    fn rejects_unsupported_uniform_resource() {
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
            Err("unsupported shader resource @group(0) @binding(2)".into())
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

    fn mesh_with_position_format(format: wgpu::VertexFormat) -> belfast::Mesh {
        belfast::Mesh::new(3)
            .unwrap()
            .add_vertex_buffer_layout(belfast::VertexBufferLayoutDescriptor {
                array_stride: format.size(),
                attributes: vec![belfast::VertexAttributeDescriptor {
                    shader_location: 0,
                    format,
                    offset: 0,
                }],
                slot: Some(0),
                step_mode: Some(wgpu::VertexStepMode::Vertex),
            })
            .unwrap()
    }
}
