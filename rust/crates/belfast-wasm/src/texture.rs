#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::{to_js_error, WasmDevice};

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextureOptionsInput {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    flip_y: Option<bool>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    mag_filter: Option<String>,
    #[serde(default)]
    min_filter: Option<String>,
}

#[cfg(any(target_arch = "wasm32", test))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct TextureOptions {
    label: String,
    flip_y: bool,
    format: wgpu::TextureFormat,
    mag_filter: wgpu::FilterMode,
    min_filter: wgpu::FilterMode,
}

#[cfg(any(target_arch = "wasm32", test))]
impl TextureOptionsInput {
    fn resolve(self) -> Result<TextureOptions, String> {
        Ok(TextureOptions {
            label: self.label.unwrap_or_else(|| "Texture".into()),
            flip_y: self.flip_y.unwrap_or(true),
            format: resolve_format(self.format.as_deref())?,
            mag_filter: resolve_filter("magFilter", self.mag_filter.as_deref())?,
            min_filter: resolve_filter("minFilter", self.min_filter.as_deref())?,
        })
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn resolve_format(value: Option<&str>) -> Result<wgpu::TextureFormat, String> {
    match value.unwrap_or("rgba8unorm-srgb") {
        "rgba8unorm" => Ok(wgpu::TextureFormat::Rgba8Unorm),
        "rgba8unorm-srgb" => Ok(wgpu::TextureFormat::Rgba8UnormSrgb),
        value => Err(format!("unsupported texture format \"{value}\"")),
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn resolve_filter(name: &str, value: Option<&str>) -> Result<wgpu::FilterMode, String> {
    match value.unwrap_or("linear") {
        "nearest" => Ok(wgpu::FilterMode::Nearest),
        "linear" => Ok(wgpu::FilterMode::Linear),
        value => Err(format!("unsupported texture {name} \"{value}\"")),
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct TextureState {
    texture: belfast::Texture,
}

#[cfg(target_arch = "wasm32")]
impl TextureState {
    pub(crate) fn texture(&self) -> &belfast::Texture {
        &self.texture
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = Texture)]
pub struct WasmTexture {
    pub(crate) state: Rc<TextureState>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = Texture)]
impl WasmTexture {
    #[wasm_bindgen(js_name = fromImageBitmap)]
    pub fn from_image_bitmap(
        device: &WasmDevice,
        bitmap: &web_sys::ImageBitmap,
        options: Option<JsValue>,
    ) -> Result<WasmTexture, JsValue> {
        let options: TextureOptionsInput = options
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(to_js_error)?
            .unwrap_or_default();
        let options = options.resolve().map_err(to_js_error)?;
        let width = bitmap.width();
        let height = bitmap.height();
        let texture = belfast::Texture::create_2d(
            &device.inner,
            width,
            height,
            belfast::TextureOptions {
                label: options.label,
                format: options.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                mag_filter: options.mag_filter,
                min_filter: options.min_filter,
            },
        )
        .map_err(to_js_error)?;

        device.inner.queue().copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(bitmap.clone()),
                origin: wgpu::Origin2d::ZERO,
                flip_y: options.flip_y,
            },
            wgpu::CopyExternalImageDestInfo {
                texture: texture.gpu(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(Self {
            state: Rc::new(TextureState { texture }),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.state.texture.width()
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.state.texture.height()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_options_default_to_external_image_settings() {
        let options = TextureOptionsInput::default().resolve().unwrap();
        assert_eq!(options.format, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(options.mag_filter, wgpu::FilterMode::Linear);
        assert!(options.flip_y);
    }

    #[test]
    fn texture_options_reject_unknown_format() {
        let options = TextureOptionsInput {
            format: Some("bgra8unorm".into()),
            ..Default::default()
        };

        assert!(options.resolve().is_err());
    }

    #[test]
    fn texture_options_reject_unknown_mag_filter() {
        let options = TextureOptionsInput {
            mag_filter: Some("cubic".into()),
            ..Default::default()
        };

        assert!(options.resolve().is_err());
    }

    #[test]
    fn texture_options_reject_unknown_min_filter() {
        let options = TextureOptionsInput {
            min_filter: Some("cubic".into()),
            ..Default::default()
        };

        assert!(options.resolve().is_err());
    }
}
