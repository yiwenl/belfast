#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(any(target_arch = "wasm32", test))]
use crate::compute::{ComputeBindingLayout, ComputeBufferType};
#[cfg(any(target_arch = "wasm32", test))]
use crate::draw::ShaderResourceLayout;
#[cfg(target_arch = "wasm32")]
use crate::{
    compute::{ComputeState, WasmCompute},
    draw::DrawState,
    texture::TextureState,
    to_js_error, WasmBuffer, WasmDevice, WasmDraw, WasmRenderTarget, WasmTexture,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{convert::TryFromJsValue, JsCast};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(typescript_custom_section)]
const BIND_GROUP_BUFFER_TYPES: &str = r#"
export interface BindGroupBufferEntry {
  binding: number;
  buffer: Buffer;
  offset?: number;
  size?: number;
}

export interface BindGroupBufferOptions {
  label?: string;
}
"#;

#[cfg(target_arch = "wasm32")]
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BindGroupOptionsInput {
    #[serde(default)]
    group_index: Option<u32>,
    #[serde(default)]
    texture_binding: Option<u32>,
    #[serde(default)]
    sampler_binding: Option<u32>,
    #[serde(default)]
    label: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UniformBindGroupOptionsInput {
    #[serde(default)]
    group_index: Option<u32>,
    #[serde(default)]
    binding: Option<u32>,
    #[serde(default)]
    label: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BindGroupBufferOptionsInput {
    #[serde(default)]
    label: Option<String>,
}

#[cfg(target_arch = "wasm32")]
struct UniformBindGroupOptions {
    group_index: u32,
    binding: u32,
    label: String,
}

#[cfg(target_arch = "wasm32")]
impl UniformBindGroupOptionsInput {
    fn resolve(self) -> UniformBindGroupOptions {
        UniformBindGroupOptions {
            group_index: self.group_index.unwrap_or(0),
            binding: self.binding.unwrap_or(0),
            label: self.label.unwrap_or_else(|| "UniformBindGroup".into()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct BindGroupOptions {
    group_index: u32,
    texture_binding: u32,
    sampler_binding: u32,
    label: String,
}

#[cfg(target_arch = "wasm32")]
impl BindGroupOptionsInput {
    fn resolve(self, default_label: &str) -> BindGroupOptions {
        BindGroupOptions {
            group_index: self.group_index.unwrap_or(0),
            texture_binding: self.texture_binding.unwrap_or(0),
            sampler_binding: self.sampler_binding.unwrap_or(1),
            label: self.label.unwrap_or_else(|| default_label.into()),
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn validate_texture_bindings(
    required: &ShaderResourceLayout,
    group_index: u32,
    texture_binding: u32,
    sampler_binding: u32,
) -> Result<(), String> {
    match required {
        ShaderResourceLayout::TextureSampler {
            group,
            texture_binding: required_texture,
            sampler_binding: required_sampler,
        } if *group == group_index
            && *required_texture == texture_binding
            && *required_sampler == sampler_binding => Ok(()),
        ShaderResourceLayout::TextureSampler {
            group,
            texture_binding,
            sampler_binding,
        } => Err(format!(
            "bind group bindings must match draw shader layout @group({group}) texture @binding({texture_binding}) sampler @binding({sampler_binding})"
        )),
        ShaderResourceLayout::None | ShaderResourceLayout::Uniform { .. } => {
            Err("draw shader does not declare a sampled texture and sampler".into())
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn validate_uniform_bindings(
    required: &ShaderResourceLayout,
    group_index: u32,
    binding: u32,
) -> Result<(), String> {
    match required {
        ShaderResourceLayout::Uniform {
            group,
            binding: required_binding,
        } if *group == group_index && *required_binding == binding => Ok(()),
        ShaderResourceLayout::Uniform { group, binding } => Err(format!(
            "bind group bindings must match draw shader layout @group({group}) uniform @binding({binding})"
        )),
        ShaderResourceLayout::None | ShaderResourceLayout::TextureSampler { .. } => {
            Err("draw shader does not declare a uniform buffer".into())
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferBindingSpec {
    binding: u32,
    buffer_size: u64,
    buffer_usage: wgpu::BufferUsages,
    offset: u64,
    size: Option<u64>,
}

#[cfg(any(target_arch = "wasm32", test))]
fn validate_compute_buffer_bindings(
    layout: &[ComputeBindingLayout],
    entries: &[BufferBindingSpec],
    min_uniform_align: u64,
    min_storage_align: u64,
) -> Result<(), String> {
    if layout.is_empty() {
        return Err("compute does not declare buffer bindings".into());
    }

    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.binding) {
            return Err(format!("duplicate bind group binding {}", entry.binding));
        }
        let Some(layout_entry) = layout.iter().find(|item| item.binding == entry.binding) else {
            return Err(format!(
                "bind group has unexpected binding {}",
                entry.binding
            ));
        };
        if !entry
            .buffer_usage
            .contains(layout_entry.ty.required_usage())
        {
            return Err(format!(
                "buffer at binding {} must have {} usage",
                entry.binding,
                layout_entry.ty.as_str()
            ));
        }
        let alignment = match layout_entry.ty {
            ComputeBufferType::Uniform => min_uniform_align,
            ComputeBufferType::ReadOnlyStorage | ComputeBufferType::Storage => min_storage_align,
        };
        if alignment == 0 || entry.offset % alignment != 0 {
            return Err(format!(
                "buffer offset at binding {} must be a multiple of {alignment}",
                entry.binding
            ));
        }
        if entry.offset > entry.buffer_size {
            return Err(format!(
                "buffer offset at binding {} exceeds buffer size",
                entry.binding
            ));
        }
        let remaining = entry.buffer_size - entry.offset;
        let bound_size = match entry.size {
            Some(0) => {
                return Err(format!(
                    "buffer size at binding {} must be greater than 0",
                    entry.binding
                ));
            }
            Some(size) => {
                if size > remaining {
                    return Err(format!(
                        "buffer binding at binding {} exceeds buffer size",
                        entry.binding
                    ));
                }
                size
            }
            None => remaining,
        };
        if bound_size == 0 {
            return Err(format!(
                "buffer binding at binding {} is empty",
                entry.binding
            ));
        }
        if let Some(min_size) = layout_entry.min_binding_size {
            if bound_size < min_size {
                return Err(format!(
                    "buffer at binding {} is smaller than minBindingSize {min_size}",
                    entry.binding
                ));
            }
        }
        if layout_entry.ty == ComputeBufferType::Uniform && bound_size % 16 != 0 {
            return Err(format!(
                "uniform buffer size at binding {} must be a multiple of 16",
                entry.binding
            ));
        }
    }

    for layout_entry in layout {
        if !seen.contains(&layout_entry.binding) {
            return Err(format!(
                "bind group is missing binding {}",
                layout_entry.binding
            ));
        }
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
struct ParsedBufferEntry {
    binding: u32,
    buffer: belfast::Buffer,
    offset: u64,
    size: Option<u64>,
}

#[cfg(target_arch = "wasm32")]
fn parse_js_buffer_bind_entries(value: &JsValue) -> Result<Vec<ParsedBufferEntry>, JsValue> {
    if !js_sys::Array::is_array(value) {
        return Err(to_js_error("bind group entries must be an array"));
    }
    let array = js_sys::Array::from(value);
    let mut entries = Vec::with_capacity(array.length() as usize);
    for index in 0..array.length() {
        let entry = array.get(index);
        let object = entry
            .dyn_ref::<js_sys::Object>()
            .ok_or_else(|| to_js_error("bind group entry must be an object"))?;
        let binding = required_u32_field(object, "binding")?;
        let buffer_value = js_sys::Reflect::get(object, &JsValue::from_str("buffer"))
            .map_err(|_| to_js_error("bind group entry buffer must be a live Buffer"))?;
        let buffer_handle = crate::frame::clone_live_wasm_class(
            &buffer_value,
            "bind group entry buffer must be a live Buffer",
        )?;
        let buffer = WasmBuffer::try_from_js_value(buffer_handle)
            .map_err(|_| to_js_error("bind group entry buffer must be a live Buffer"))?;
        let offset = optional_u64_field(object, "offset")?.unwrap_or(0);
        let size = optional_u64_field(object, "size")?;
        entries.push(ParsedBufferEntry {
            binding,
            buffer: buffer.inner().clone(),
            offset,
            size,
        });
    }
    Ok(entries)
}

#[cfg(target_arch = "wasm32")]
fn required_u32_field(object: &js_sys::Object, name: &str) -> Result<u32, JsValue> {
    let value = js_sys::Reflect::get(object, &JsValue::from_str(name))
        .map_err(|_| to_js_error(format!("bind group entry {name} must be a number")))?;
    parse_u32(&value, &format!("bind group entry {name}")).map_err(to_js_error)
}

#[cfg(target_arch = "wasm32")]
fn optional_u64_field(object: &js_sys::Object, name: &str) -> Result<Option<u64>, JsValue> {
    let value = js_sys::Reflect::get(object, &JsValue::from_str(name))
        .map_err(|_| to_js_error(format!("bind group entry {name} must be a number")))?;
    if value.is_undefined() {
        return Ok(None);
    }
    parse_u64(&value, &format!("bind group entry {name}"))
        .map(Some)
        .map_err(to_js_error)
}

#[cfg(target_arch = "wasm32")]
fn parse_u32(value: &JsValue, name: &str) -> Result<u32, String> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{name} must be a number"))?;
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > f64::from(u32::MAX)
    {
        return Err(format!("{name} must be a non-negative integer"));
    }
    Ok(number as u32)
}

#[cfg(target_arch = "wasm32")]
fn parse_u64(value: &JsValue, name: &str) -> Result<u64, String> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{name} must be a number"))?;
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 {
        return Err(format!("{name} must be a non-negative integer"));
    }
    Ok(number as u64)
}

#[cfg(target_arch = "wasm32")]
enum BindGroupOwner {
    Draw(Rc<DrawState>),
    Compute(Rc<ComputeState>),
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(crate) struct BindGroupState {
    bind_group: belfast::BindGroup,
    owner: BindGroupOwner,
    group_index: u32,
    source: BindGroupSource,
}

#[cfg(target_arch = "wasm32")]
impl BindGroupState {
    pub(crate) fn bind_group(&self) -> &belfast::BindGroup {
        &self.bind_group
    }

    pub(crate) fn draw(&self) -> Option<&Rc<DrawState>> {
        match &self.owner {
            BindGroupOwner::Draw(draw) => Some(draw),
            BindGroupOwner::Compute(_) => None,
        }
    }

    pub(crate) fn compute(&self) -> Option<&Rc<ComputeState>> {
        match &self.owner {
            BindGroupOwner::Compute(compute) => Some(compute),
            BindGroupOwner::Draw(_) => None,
        }
    }

    pub(crate) fn group_index(&self) -> u32 {
        self.group_index
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
enum BindGroupSource {
    Texture(Rc<TextureState>),
    RenderTarget(Rc<RefCell<belfast::RenderTarget>>),
    Buffer(belfast::Buffer),
    Buffers(Vec<belfast::Buffer>),
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = BindGroup)]
pub struct WasmBindGroup {
    #[allow(dead_code)]
    pub(crate) state: Rc<BindGroupState>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = BindGroup)]
impl WasmBindGroup {
    #[wasm_bindgen(js_name = fromTexture, unchecked_return_type = "BindGroup")]
    pub fn from_texture(
        device: &WasmDevice,
        draw: &WasmDraw,
        texture: &WasmTexture,
        options: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let options: BindGroupOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let options = options.resolve("TextureBindGroup");
        validate_texture_bindings(
            &draw.state.resources,
            options.group_index,
            options.texture_binding,
            options.sampler_binding,
        )
        .map_err(to_js_error)?;

        if !draw.state.draw().device().is_same(&device.inner) {
            return Err(to_js_error("draw was created by a different device"));
        }
        if !texture.state.texture().device().is_same(&device.inner) {
            return Err(to_js_error("texture was created by a different device"));
        }

        let layout = draw.state.draw().get_bind_group_layout(options.group_index);
        let bind_group = belfast::BindGroup::create(
            &device.inner,
            &layout,
            &[
                wgpu::BindGroupEntry {
                    binding: options.texture_binding,
                    resource: wgpu::BindingResource::TextureView(texture.state.texture().view()),
                },
                wgpu::BindGroupEntry {
                    binding: options.sampler_binding,
                    resource: wgpu::BindingResource::Sampler(texture.state.texture().sampler()),
                },
            ],
            &options.label,
        );

        let wrapper = JsValue::from(Self {
            state: Rc::new(BindGroupState {
                bind_group,
                owner: BindGroupOwner::Draw(draw.state.clone()),
                group_index: options.group_index,
                source: BindGroupSource::Texture(texture.state.clone()),
            }),
        });
        crate::frame::register_bind_group_wrapper(&wrapper)?;
        Ok(wrapper)
    }

    #[wasm_bindgen(js_name = fromRenderTarget, unchecked_return_type = "BindGroup")]
    pub fn from_render_target(
        device: &WasmDevice,
        draw: &WasmDraw,
        render_target: &WasmRenderTarget,
        options: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let options: BindGroupOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let options = options.resolve("RenderTargetBindGroup");
        validate_texture_bindings(
            &draw.state.resources,
            options.group_index,
            options.texture_binding,
            options.sampler_binding,
        )
        .map_err(to_js_error)?;

        if !draw.state.draw().device().is_same(&device.inner) {
            return Err(to_js_error("draw was created by a different device"));
        }
        if !render_target.device.is_same(&device.inner) {
            return Err(to_js_error(
                "render target was created by a different device",
            ));
        }

        let layout = draw.state.draw().get_bind_group_layout(options.group_index);
        let bind_group = {
            let target = render_target.target.borrow();
            belfast::BindGroup::create(
                &device.inner,
                &layout,
                &[
                    wgpu::BindGroupEntry {
                        binding: options.texture_binding,
                        resource: wgpu::BindingResource::TextureView(target.color_view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: options.sampler_binding,
                        resource: wgpu::BindingResource::Sampler(target.sampler()),
                    },
                ],
                &options.label,
            )
        };

        let wrapper = JsValue::from(Self {
            state: Rc::new(BindGroupState {
                bind_group,
                owner: BindGroupOwner::Draw(draw.state.clone()),
                group_index: options.group_index,
                source: BindGroupSource::RenderTarget(render_target.target.clone()),
            }),
        });
        crate::frame::register_bind_group_wrapper(&wrapper)?;
        Ok(wrapper)
    }

    #[wasm_bindgen(js_name = fromBuffer, unchecked_return_type = "BindGroup")]
    pub fn from_buffer(
        device: &WasmDevice,
        draw: &WasmDraw,
        buffer: &WasmBuffer,
        options: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let options: UniformBindGroupOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let options = options.resolve();
        validate_uniform_bindings(&draw.state.resources, options.group_index, options.binding)
            .map_err(to_js_error)?;

        if !draw.state.draw().device().is_same(&device.inner) {
            return Err(to_js_error("draw was created by a different device"));
        }
        if !buffer.inner().device().is_same(&device.inner) {
            return Err(to_js_error("buffer was created by a different device"));
        }

        let layout = draw.state.draw().get_bind_group_layout(options.group_index);
        let bind_group = belfast::BindGroup::create(
            &device.inner,
            &layout,
            &[wgpu::BindGroupEntry {
                binding: options.binding,
                resource: buffer.inner().gpu().as_entire_binding(),
            }],
            &options.label,
        );

        let wrapper = JsValue::from(Self {
            state: Rc::new(BindGroupState {
                bind_group,
                owner: BindGroupOwner::Draw(draw.state.clone()),
                group_index: options.group_index,
                source: BindGroupSource::Buffer(buffer.inner().clone()),
            }),
        });
        crate::frame::register_bind_group_wrapper(&wrapper)?;
        Ok(wrapper)
    }

    #[wasm_bindgen(js_name = fromBuffers, unchecked_return_type = "BindGroup")]
    pub fn from_buffers(
        device: &WasmDevice,
        compute: &WasmCompute,
        #[wasm_bindgen(unchecked_param_type = "BindGroupBufferEntry[]")] entries: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "BindGroupBufferOptions")] options: Option<
            JsValue,
        >,
    ) -> Result<JsValue, JsValue> {
        let options: BindGroupBufferOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let label = options.label.unwrap_or_else(|| "ComputeBindGroup".into());
        let parsed_entries = parse_js_buffer_bind_entries(&entries)?;
        let specs: Vec<_> = parsed_entries
            .iter()
            .map(|entry| BufferBindingSpec {
                binding: entry.binding,
                buffer_size: entry.buffer.size(),
                buffer_usage: entry.buffer.usage(),
                offset: entry.offset,
                size: entry.size,
            })
            .collect();
        let limits = device.inner.gpu().limits();
        validate_compute_buffer_bindings(
            compute.state.layout(),
            &specs,
            u64::from(limits.min_uniform_buffer_offset_alignment),
            u64::from(limits.min_storage_buffer_offset_alignment),
        )
        .map_err(to_js_error)?;

        if !compute.state.compute().device().is_same(&device.inner) {
            return Err(to_js_error("compute was created by a different device"));
        }
        for entry in &parsed_entries {
            if !entry.buffer.device().is_same(&device.inner) {
                return Err(to_js_error("buffer was created by a different device"));
            }
        }

        let layout = compute.state.compute().get_bind_group_layout(0);
        let gpu_entries: Vec<_> = parsed_entries
            .iter()
            .map(|entry| wgpu::BindGroupEntry {
                binding: entry.binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: entry.buffer.gpu(),
                    offset: entry.offset,
                    size: entry.size.and_then(std::num::NonZeroU64::new),
                }),
            })
            .collect();
        let bind_group = belfast::BindGroup::create(&device.inner, &layout, &gpu_entries, &label);
        let buffers: Vec<_> = parsed_entries
            .into_iter()
            .map(|entry| entry.buffer)
            .collect();

        let wrapper = JsValue::from(Self {
            state: Rc::new(BindGroupState {
                bind_group,
                owner: BindGroupOwner::Compute(compute.state.clone()),
                group_index: 0,
                source: BindGroupSource::Buffers(buffers),
            }),
        });
        crate::frame::register_bind_group_wrapper(&wrapper)?;
        Ok(wrapper)
    }

    #[wasm_bindgen(js_name = __frameHandle, skip_typescript)]
    pub fn frame_handle(&self) -> WasmBindGroup {
        Self {
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_group_bindings_must_match_draw_shader_layout() {
        let required = ShaderResourceLayout::TextureSampler {
            group: 0,
            texture_binding: 0,
            sampler_binding: 1,
        };
        assert!(validate_texture_bindings(&required, 0, 0, 1).is_ok());
        assert!(validate_texture_bindings(&required, 0, 1, 0).is_err());
        assert_eq!(
            validate_texture_bindings(
                &ShaderResourceLayout::Uniform {
                    group: 0,
                    binding: 0,
                },
                0,
                0,
                1
            ),
            Err("draw shader does not declare a sampled texture and sampler".into())
        );
    }

    #[test]
    fn uniform_bind_group_bindings_must_match_draw_shader_layout() {
        let required = ShaderResourceLayout::Uniform {
            group: 0,
            binding: 0,
        };
        assert!(validate_uniform_bindings(&required, 0, 0).is_ok());
        assert_eq!(
            validate_uniform_bindings(&required, 0, 1).unwrap_err(),
            "bind group bindings must match draw shader layout @group(0) uniform @binding(0)"
        );
        assert_eq!(
            validate_uniform_bindings(&ShaderResourceLayout::None, 0, 0).unwrap_err(),
            "draw shader does not declare a uniform buffer"
        );
    }

    fn layout_entry(binding: u32, ty: ComputeBufferType) -> ComputeBindingLayout {
        ComputeBindingLayout {
            binding,
            ty,
            min_binding_size: None,
        }
    }

    fn spec(
        binding: u32,
        buffer_size: u64,
        buffer_usage: wgpu::BufferUsages,
        offset: u64,
        size: Option<u64>,
    ) -> BufferBindingSpec {
        BufferBindingSpec {
            binding,
            buffer_size,
            buffer_usage,
            offset,
            size,
        }
    }

    #[test]
    fn compute_bind_group_entries_must_match_layout() {
        let layout = [
            layout_entry(0, ComputeBufferType::Uniform),
            layout_entry(1, ComputeBufferType::Storage),
        ];
        let entries = [
            spec(0, 256, wgpu::BufferUsages::UNIFORM, 0, None),
            spec(1, 64, wgpu::BufferUsages::STORAGE, 0, None),
        ];
        assert!(validate_compute_buffer_bindings(&layout, &entries, 256, 256).is_ok());
        assert_eq!(
            validate_compute_buffer_bindings(&layout, &entries[..1], 256, 256).unwrap_err(),
            "bind group is missing binding 1"
        );
        assert_eq!(
            validate_compute_buffer_bindings(
                &layout,
                &[
                    spec(0, 256, wgpu::BufferUsages::UNIFORM, 0, None),
                    spec(2, 64, wgpu::BufferUsages::STORAGE, 0, None),
                ],
                256,
                256
            )
            .unwrap_err(),
            "bind group has unexpected binding 2"
        );
        assert_eq!(
            validate_compute_buffer_bindings(
                &layout,
                &[
                    spec(0, 256, wgpu::BufferUsages::STORAGE, 0, None),
                    spec(1, 64, wgpu::BufferUsages::STORAGE, 0, None),
                ],
                256,
                256
            )
            .unwrap_err(),
            "buffer at binding 0 must have uniform usage"
        );
        assert_eq!(
            validate_compute_buffer_bindings(&[], &[], 256, 256).unwrap_err(),
            "compute does not declare buffer bindings"
        );
    }

    #[test]
    fn compute_bind_group_rejects_invalid_offset_and_size() {
        let layout = [ComputeBindingLayout {
            binding: 0,
            ty: ComputeBufferType::Storage,
            min_binding_size: Some(32),
        }];
        assert_eq!(
            validate_compute_buffer_bindings(
                &layout,
                &[spec(0, 64, wgpu::BufferUsages::STORAGE, 8, None)],
                256,
                256
            )
            .unwrap_err(),
            "buffer offset at binding 0 must be a multiple of 256"
        );
        assert_eq!(
            validate_compute_buffer_bindings(
                &layout,
                &[spec(0, 16, wgpu::BufferUsages::STORAGE, 0, None)],
                256,
                256
            )
            .unwrap_err(),
            "buffer at binding 0 is smaller than minBindingSize 32"
        );
        assert_eq!(
            validate_compute_buffer_bindings(
                &layout,
                &[spec(0, 64, wgpu::BufferUsages::STORAGE, 0, Some(0))],
                256,
                256
            )
            .unwrap_err(),
            "buffer size at binding 0 must be greater than 0"
        );
    }
}
