use wasm_bindgen::prelude::*;

use crate::{slice_to_vec3, to_js_error};

#[wasm_bindgen(js_name = PerspectiveCamera)]
pub struct WasmPerspectiveCamera {
    inner: belfast::PerspectiveCamera,
}

impl WasmPerspectiveCamera {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn inner_mut(&mut self) -> &mut belfast::PerspectiveCamera {
        &mut self.inner
    }
}

#[wasm_bindgen(js_class = PerspectiveCamera)]
impl WasmPerspectiveCamera {
    #[wasm_bindgen(constructor)]
    pub fn new(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            inner: belfast::PerspectiveCamera::new(fovy_radians, aspect, near, far),
        }
    }

    #[wasm_bindgen(js_name = lookAt)]
    pub fn look_at(&mut self, eye: &[f32], target: &[f32]) -> Result<(), JsValue> {
        self.inner
            .look_at(slice_to_vec3("eye", eye)?, slice_to_vec3("target", target)?);
        Ok(())
    }

    #[wasm_bindgen(js_name = setAspect)]
    pub fn set_aspect(&mut self, aspect: f32) -> Result<(), JsValue> {
        self.inner
            .set_aspect(aspect)
            .map(|_| ())
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = setFieldOfView)]
    pub fn set_field_of_view(&mut self, fovy_radians: f32) -> Result<(), JsValue> {
        self.inner
            .set_fovy_radians(fovy_radians)
            .map(|_| ())
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = getPosition)]
    pub fn position(&self) -> Vec<f32> {
        self.inner.position().to_vec()
    }

    #[wasm_bindgen(js_name = getFieldOfView)]
    pub fn field_of_view(&self) -> f32 {
        self.inner.fovy_radians()
    }

    #[wasm_bindgen(js_name = getAspect)]
    pub fn aspect(&self) -> f32 {
        self.inner.aspect()
    }

    #[wasm_bindgen(js_name = getNear)]
    pub fn near(&self) -> f32 {
        self.inner.near()
    }

    #[wasm_bindgen(js_name = getFar)]
    pub fn far(&self) -> f32 {
        self.inner.far()
    }

    #[wasm_bindgen(js_name = getViewMatrix)]
    pub fn view_matrix(&self) -> Vec<f32> {
        self.inner.view_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getProjectionMatrix)]
    pub fn projection_matrix(&self) -> Vec<f32> {
        self.inner.projection_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getViewProjectionMatrix)]
    pub fn view_projection_matrix(&self) -> Vec<f32> {
        self.inner.view_projection_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getLookAtTarget)]
    pub fn look_at_target(&self) -> Vec<f32> {
        self.inner.look_at_target().to_vec()
    }
}

#[wasm_bindgen(js_name = OrthographicCamera)]
pub struct WasmOrthographicCamera {
    inner: belfast::OrthographicCamera,
}

#[wasm_bindgen(js_class = OrthographicCamera)]
impl WasmOrthographicCamera {
    #[wasm_bindgen(constructor)]
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self {
            inner: belfast::OrthographicCamera::new(left, right, bottom, top, near, far),
        }
    }

    #[wasm_bindgen(js_name = lookAt)]
    pub fn look_at(&mut self, eye: &[f32], target: &[f32]) -> Result<(), JsValue> {
        self.inner
            .look_at(slice_to_vec3("eye", eye)?, slice_to_vec3("target", target)?);
        Ok(())
    }

    #[wasm_bindgen(js_name = getPosition)]
    pub fn position(&self) -> Vec<f32> {
        self.inner.position().to_vec()
    }

    #[wasm_bindgen(js_name = getViewMatrix)]
    pub fn view_matrix(&self) -> Vec<f32> {
        self.inner.view_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getProjectionMatrix)]
    pub fn projection_matrix(&self) -> Vec<f32> {
        self.inner.projection_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getViewProjectionMatrix)]
    pub fn view_projection_matrix(&self) -> Vec<f32> {
        self.inner.view_projection_matrix().to_vec()
    }

    #[wasm_bindgen(js_name = getLookAtTarget)]
    pub fn look_at_target(&self) -> Vec<f32> {
        self.inner.look_at_target().to_vec()
    }
}
