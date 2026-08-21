use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{to_js_error, WasmDevice};

#[wasm_bindgen(typescript_custom_section)]
const COMPUTE_TYPES: &str = r#"
export type ComputeBindingType = "uniform" | "readOnlyStorage" | "storage";

export interface ComputeLayoutEntry {
  binding: number;
  type: ComputeBindingType;
  minBindingSize?: number;
}

export interface ComputeOptions {
  label?: string;
  entryPoint?: string;
  layout: ComputeLayoutEntry[];
}

export type WorkgroupCount = number | readonly [number, number] | readonly [number, number, number];
"#;

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ComputeOptionsInput {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    entry_point: Option<String>,
    layout: Vec<ComputeLayoutEntryInput>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ComputeLayoutEntryInput {
    binding: u32,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    min_binding_size: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComputeBufferType {
    Uniform,
    ReadOnlyStorage,
    Storage,
}

impl ComputeBufferType {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "uniform" => Ok(Self::Uniform),
            "readOnlyStorage" => Ok(Self::ReadOnlyStorage),
            "storage" => Ok(Self::Storage),
            _ => Err(format!("unsupported compute binding type \"{value}\"")),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::ReadOnlyStorage => "read-only storage",
            Self::Storage => "storage",
        }
    }

    fn binding_type(self) -> wgpu::BufferBindingType {
        match self {
            Self::Uniform => wgpu::BufferBindingType::Uniform,
            Self::ReadOnlyStorage => wgpu::BufferBindingType::Storage { read_only: true },
            Self::Storage => wgpu::BufferBindingType::Storage { read_only: false },
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn required_usage(self) -> wgpu::BufferUsages {
        match self {
            Self::Uniform => wgpu::BufferUsages::UNIFORM,
            Self::ReadOnlyStorage | Self::Storage => wgpu::BufferUsages::STORAGE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComputeBindingLayout {
    pub binding: u32,
    pub ty: ComputeBufferType,
    pub min_binding_size: Option<u64>,
}

impl ComputeBindingLayout {
    fn from_input(input: ComputeLayoutEntryInput) -> Result<Self, String> {
        if input.min_binding_size == Some(0) {
            return Err(format!(
                "compute layout binding {} minBindingSize must be greater than 0",
                input.binding
            ));
        }
        Ok(Self {
            binding: input.binding,
            ty: ComputeBufferType::parse(&input.ty)?,
            min_binding_size: input.min_binding_size,
        })
    }

    fn bind_group_layout_entry(self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: self.ty.binding_type(),
                has_dynamic_offset: false,
                min_binding_size: self.min_binding_size.and_then(std::num::NonZeroU64::new),
            },
            count: None,
        }
    }
}

fn parse_compute_layout(
    entries: Vec<ComputeLayoutEntryInput>,
) -> Result<Vec<ComputeBindingLayout>, String> {
    let mut seen = BTreeSet::new();
    let mut layout = Vec::with_capacity(entries.len());
    for entry in entries {
        let parsed = ComputeBindingLayout::from_input(entry)?;
        if !seen.insert(parsed.binding) {
            return Err(format!(
                "duplicate compute layout binding {}",
                parsed.binding
            ));
        }
        layout.push(parsed);
    }
    layout.sort_by_key(|entry| entry.binding);
    Ok(layout)
}

fn parse_and_validate_compute_shader(
    shader_code: &str,
    entry_point: &str,
    layout: &[ComputeBindingLayout],
) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(shader_code)
        .map_err(|error| format!("WGSL parse failed: {error}"))?;
    validate_compute_interface(&module, entry_point, layout)?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|error| format!("WGSL validation failed: {error}"))?;
    Ok(())
}

fn validate_compute_interface(
    module: &naga::Module,
    entry_point: &str,
    layout: &[ComputeBindingLayout],
) -> Result<(), String> {
    if !module
        .entry_points
        .iter()
        .any(|entry| entry.name == entry_point && entry.stage == naga::ShaderStage::Compute)
    {
        return Err(format!(
            "shader is missing required compute entry point \"{entry_point}\""
        ));
    }

    let shader_bindings = collect_compute_shader_bindings(module)?;
    let layout_bindings: BTreeMap<u32, ComputeBufferType> = layout
        .iter()
        .map(|entry| (entry.binding, entry.ty))
        .collect();

    for (binding, ty) in &shader_bindings {
        let Some(expected) = layout_bindings.get(binding) else {
            return Err(format!(
                "shader resource @group(0) @binding({binding}) is missing from compute layout"
            ));
        };
        if ty != expected {
            return Err(format!(
                "shader resource @group(0) @binding({binding}) is a {}, expected {}",
                ty.as_str(),
                expected.as_str()
            ));
        }
    }

    for binding in layout_bindings.keys() {
        if !shader_bindings.contains_key(binding) {
            return Err(format!(
                "compute layout binding {binding} is not declared in the shader"
            ));
        }
    }

    Ok(())
}

fn collect_compute_shader_bindings(
    module: &naga::Module,
) -> Result<BTreeMap<u32, ComputeBufferType>, String> {
    let mut bindings = BTreeMap::new();
    for (_, variable) in module.global_variables.iter() {
        let Some(binding) = variable.binding else {
            continue;
        };
        if binding.group != 0 {
            return Err(unsupported_compute_resource(binding));
        }
        let ty = match variable.space {
            naga::AddressSpace::Uniform => ComputeBufferType::Uniform,
            naga::AddressSpace::Storage { access } => {
                if access.contains(naga::StorageAccess::STORE)
                    || access.contains(naga::StorageAccess::ATOMIC)
                {
                    ComputeBufferType::Storage
                } else {
                    ComputeBufferType::ReadOnlyStorage
                }
            }
            _ => return Err(unsupported_compute_resource(binding)),
        };
        if bindings.insert(binding.binding, ty).is_some() {
            return Err(format!(
                "duplicate shader resource @group(0) @binding({})",
                binding.binding
            ));
        }
    }
    Ok(bindings)
}

fn unsupported_compute_resource(binding: naga::ResourceBinding) -> String {
    format!(
        "unsupported compute shader resource @group({}) @binding({})",
        binding.group, binding.binding
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn parse_workgroups(value: &JsValue, max: u32) -> Result<[u32; 3], String> {
    if value.is_undefined() || value.is_null() {
        return validate_workgroup_count(1.0, max).map(|x| [x, 1, 1]);
    }
    if let Some(count) = value.as_f64() {
        return validate_workgroup_count(count, max).map(|x| [x, 1, 1]);
    }
    if !js_sys::Array::is_array(value) {
        return Err("workgroups must be a number or [x, y] or [x, y, z]".into());
    }

    let values = js_sys::Array::from(value);
    let len = values.length();
    if !(2..=3).contains(&len) {
        return Err("workgroups must be a number or [x, y] or [x, y, z]".into());
    }

    let x = validate_workgroup_count(values.get(0).as_f64().unwrap_or(f64::NAN), max)?;
    let y = validate_workgroup_count(values.get(1).as_f64().unwrap_or(f64::NAN), max)?;
    let z = if len == 3 {
        validate_workgroup_count(values.get(2).as_f64().unwrap_or(f64::NAN), max)?
    } else {
        1
    };
    Ok([x, y, z])
}

#[cfg(any(target_arch = "wasm32", test))]
fn validate_workgroup_count(value: f64, max: u32) -> Result<u32, String> {
    if !value.is_finite() || value.fract() != 0.0 || value < 1.0 {
        return Err("workgroup counts must be integers greater than 0".into());
    }
    if value > f64::from(max) {
        return Err(format!(
            "workgroup count {value} exceeds device limit {max}"
        ));
    }
    Ok(value as u32)
}

pub(crate) struct ComputeState {
    compute: belfast::Compute,
    layout: Vec<ComputeBindingLayout>,
}

impl ComputeState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn compute(&self) -> &belfast::Compute {
        &self.compute
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn layout(&self) -> &[ComputeBindingLayout] {
        &self.layout
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn requires_bind_group(&self) -> bool {
        !self.layout.is_empty()
    }
}

#[wasm_bindgen(js_name = Compute)]
pub struct WasmCompute {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) state: Rc<ComputeState>,
}

#[wasm_bindgen(js_class = Compute)]
impl WasmCompute {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        shader_code: &str,
        #[wasm_bindgen(unchecked_param_type = "ComputeOptions")] options: JsValue,
    ) -> Result<WasmCompute, JsValue> {
        let options: ComputeOptionsInput =
            serde_wasm_bindgen::from_value(options).map_err(to_js_error)?;
        let label = options.label.as_deref().unwrap_or("Compute");
        let entry_point = options.entry_point.as_deref().unwrap_or("cs_main");
        let layout = parse_compute_layout(options.layout).map_err(to_js_error)?;
        parse_and_validate_compute_shader(shader_code, entry_point, &layout)
            .map_err(to_js_error)?;

        let bind_group_layout =
            if layout.is_empty() {
                None
            } else {
                let entries: Vec<_> = layout
                    .iter()
                    .copied()
                    .map(ComputeBindingLayout::bind_group_layout_entry)
                    .collect();
                Some(device.inner.gpu().create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        label: Some(&format!("{label}BindGroupLayout")),
                        entries: &entries,
                    },
                ))
            };
        let bind_group_layouts: Vec<Option<&wgpu::BindGroupLayout>> = match &bind_group_layout {
            Some(layout) => vec![Some(layout)],
            None => Vec::new(),
        };
        let pipeline_layout =
            device
                .inner
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&format!("{label}Layout")),
                    bind_group_layouts: &bind_group_layouts,
                    immediate_size: 0,
                });
        let compute = belfast::Compute::new(
            &device.inner,
            shader_code,
            belfast::ComputeOptions {
                label,
                layout: Some(&pipeline_layout),
                entry_point,
            },
        );

        Ok(Self {
            state: Rc::new(ComputeState { compute, layout }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPUTE_BUFFERS_SHADER: &str = r#"
struct Params {
    time: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> source: array<vec4f>;
@group(0) @binding(2) var<storage, read_write> dest: array<vec4f>;

@compute @workgroup_size(1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    dest[id.x] = source[id.x] + vec4f(params.time);
}
"#;

    fn layout(entries: &[(u32, ComputeBufferType)]) -> Vec<ComputeBindingLayout> {
        entries
            .iter()
            .map(|(binding, ty)| ComputeBindingLayout {
                binding: *binding,
                ty: *ty,
                min_binding_size: None,
            })
            .collect()
    }

    #[test]
    fn parse_layout_rejects_duplicates_and_unknown_types() {
        let duplicate = parse_compute_layout(vec![
            ComputeLayoutEntryInput {
                binding: 0,
                ty: "uniform".into(),
                min_binding_size: None,
            },
            ComputeLayoutEntryInput {
                binding: 0,
                ty: "storage".into(),
                min_binding_size: None,
            },
        ]);
        assert_eq!(duplicate.unwrap_err(), "duplicate compute layout binding 0");
        assert_eq!(
            ComputeBufferType::parse("texture").unwrap_err(),
            "unsupported compute binding type \"texture\""
        );
    }

    #[test]
    fn accepts_matching_explicit_buffer_layout() {
        parse_and_validate_compute_shader(
            COMPUTE_BUFFERS_SHADER,
            "cs_main",
            &layout(&[
                (0, ComputeBufferType::Uniform),
                (1, ComputeBufferType::ReadOnlyStorage),
                (2, ComputeBufferType::Storage),
            ]),
        )
        .unwrap();
    }

    #[test]
    fn rejects_missing_compute_entry_point() {
        assert_eq!(
            parse_and_validate_compute_shader(
                COMPUTE_BUFFERS_SHADER,
                "main",
                &layout(&[
                    (0, ComputeBufferType::Uniform),
                    (1, ComputeBufferType::ReadOnlyStorage),
                    (2, ComputeBufferType::Storage),
                ]),
            )
            .unwrap_err(),
            "shader is missing required compute entry point \"main\""
        );
    }

    #[test]
    fn rejects_layout_mismatch() {
        assert_eq!(
            parse_and_validate_compute_shader(
                COMPUTE_BUFFERS_SHADER,
                "cs_main",
                &layout(&[
                    (0, ComputeBufferType::Uniform),
                    (1, ComputeBufferType::Storage),
                    (2, ComputeBufferType::Storage),
                ]),
            )
            .unwrap_err(),
            "shader resource @group(0) @binding(1) is a read-only storage, expected storage"
        );
        assert_eq!(
            parse_and_validate_compute_shader(
                COMPUTE_BUFFERS_SHADER,
                "cs_main",
                &layout(&[(0, ComputeBufferType::Uniform)]),
            )
            .unwrap_err(),
            "shader resource @group(0) @binding(1) is missing from compute layout"
        );
        assert_eq!(
            parse_and_validate_compute_shader(
                COMPUTE_BUFFERS_SHADER,
                "cs_main",
                &layout(&[
                    (0, ComputeBufferType::Uniform),
                    (1, ComputeBufferType::ReadOnlyStorage),
                    (2, ComputeBufferType::Storage),
                    (3, ComputeBufferType::Uniform),
                ]),
            )
            .unwrap_err(),
            "compute layout binding 3 is not declared in the shader"
        );
    }

    #[test]
    fn rejects_textures_and_other_groups() {
        const TEXTURE_SHADER: &str = r#"
@group(0) @binding(0) var image: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(1)
fn cs_main() {}
"#;
        assert_eq!(
            parse_and_validate_compute_shader(TEXTURE_SHADER, "cs_main", &[]).unwrap_err(),
            "unsupported compute shader resource @group(0) @binding(0)"
        );

        const OTHER_GROUP_SHADER: &str = r#"
@group(1) @binding(0) var<uniform> params: vec4f;

@compute @workgroup_size(1)
fn cs_main() {}
"#;
        assert_eq!(
            parse_and_validate_compute_shader(OTHER_GROUP_SHADER, "cs_main", &[]).unwrap_err(),
            "unsupported compute shader resource @group(1) @binding(0)"
        );
    }

    #[test]
    fn empty_layout_requires_shader_without_bindings() {
        const EMPTY_SHADER: &str = r#"
@compute @workgroup_size(1)
fn cs_main() {}
"#;
        parse_and_validate_compute_shader(EMPTY_SHADER, "cs_main", &[]).unwrap();
        assert_eq!(
            parse_and_validate_compute_shader(COMPUTE_BUFFERS_SHADER, "cs_main", &[]).unwrap_err(),
            "shader resource @group(0) @binding(0) is missing from compute layout"
        );
    }

    #[test]
    fn parse_workgroups_accepts_number_and_tuples() {
        assert_eq!(validate_workgroup_count(4.0, 65535).unwrap(), 4);
        assert_eq!(
            validate_workgroup_count(0.0, 65535).unwrap_err(),
            "workgroup counts must be integers greater than 0"
        );
        assert_eq!(
            validate_workgroup_count(1.5, 65535).unwrap_err(),
            "workgroup counts must be integers greater than 0"
        );
        assert_eq!(
            validate_workgroup_count(8.0, 4).unwrap_err(),
            "workgroup count 8 exceeds device limit 4"
        );
    }
}
