#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(any(target_arch = "wasm32", test))]
use crate::draw::ShaderResourceLayout;
#[cfg(target_arch = "wasm32")]
use crate::{
    draw::DrawState, texture::TextureState, to_js_error, WasmDevice, WasmDraw, WasmRenderTarget,
    WasmTexture,
};

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
        ShaderResourceLayout::None => {
            Err("draw shader does not declare a sampled texture and sampler".into())
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(crate) struct BindGroupState {
    bind_group: belfast::BindGroup,
    draw: Rc<DrawState>,
    group_index: u32,
    source: BindGroupSource,
}

#[cfg(target_arch = "wasm32")]
impl BindGroupState {
    pub(crate) fn bind_group(&self) -> &belfast::BindGroup {
        &self.bind_group
    }

    pub(crate) fn draw(&self) -> &Rc<DrawState> {
        &self.draw
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
                draw: draw.state.clone(),
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
                draw: draw.state.clone(),
                group_index: options.group_index,
                source: BindGroupSource::RenderTarget(render_target.target.clone()),
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
    }
}
