use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::{slice_to_vec3, to_js_error, WasmPerspectiveCamera};

const LINE_SCROLL_PIXELS: f32 = 16.0;
const PAGE_SCROLL_PIXELS: f32 = 100.0;

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OrbitalControlOptionsInput {
    #[serde(default)]
    center: Option<Vec<f32>>,
    #[serde(default)]
    radius: Option<f32>,
    #[serde(default)]
    min_radius: Option<f32>,
    #[serde(default)]
    max_radius: Option<f32>,
    #[serde(default)]
    rotate_sensitivity: Option<f32>,
    #[serde(default)]
    zoom_sensitivity: Option<f32>,
    #[serde(default)]
    pan_sensitivity: Option<f32>,
    #[serde(default)]
    damping: Option<f32>,
}

struct AttachedListeners {
    target: web_sys::HtmlElement,
    on_pointer_down: Closure<dyn FnMut(web_sys::PointerEvent)>,
    on_pointer_move: Closure<dyn FnMut(web_sys::PointerEvent)>,
    on_pointer_up: Closure<dyn FnMut(web_sys::PointerEvent)>,
    on_wheel: Closure<dyn FnMut(web_sys::WheelEvent)>,
}

#[wasm_bindgen(js_name = OrbitalControl)]
pub struct WasmOrbitalControl {
    inner: Rc<RefCell<belfast::OrbitalControl>>,
    listener_target: Option<web_sys::HtmlElement>,
    listeners: Option<AttachedListeners>,
}

#[wasm_bindgen(js_class = OrbitalControl)]
impl WasmOrbitalControl {
    #[wasm_bindgen(constructor)]
    pub fn new(
        camera: &mut WasmPerspectiveCamera,
        options: Option<JsValue>,
    ) -> Result<WasmOrbitalControl, JsValue> {
        let (listener_target, rust_options) = parse_options(options)?;
        let inner = belfast::OrbitalControl::new(rust_options).map_err(to_js_error)?;
        let mut control = Self {
            inner: Rc::new(RefCell::new(inner)),
            listener_target,
            listeners: None,
        };
        control.inner.borrow_mut().update(0.0, camera.inner_mut());
        if control.listener_target.is_some() {
            control.connect()?;
        }
        Ok(control)
    }

    pub fn update(&self, delta_seconds: f32, camera: &mut WasmPerspectiveCamera) -> bool {
        self.inner
            .borrow_mut()
            .update(delta_seconds, camera.inner_mut())
    }

    #[wasm_bindgen(js_name = setEnabled)]
    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), JsValue> {
        self.inner.borrow_mut().set_enabled(enabled);
        if enabled {
            self.connect()
        } else {
            self.disconnect();
            Ok(())
        }
    }

    #[wasm_bindgen(getter)]
    pub fn enabled(&self) -> bool {
        self.inner.borrow().enabled()
    }

    pub fn connect(&mut self) -> Result<(), JsValue> {
        if self.listeners.is_some() {
            return Ok(());
        }
        let Some(target) = self.listener_target.clone() else {
            return Ok(());
        };
        self.listeners = Some(attach_listeners(target, Rc::clone(&self.inner))?);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(listeners) = self.listeners.take() {
            detach_listeners(&listeners);
        }
    }

    pub fn destroy(&mut self) {
        self.disconnect();
        self.listener_target = None;
    }

    #[wasm_bindgen(js_name = getYaw)]
    pub fn yaw(&self) -> f32 {
        self.inner.borrow().yaw()
    }

    #[wasm_bindgen(js_name = setYaw)]
    pub fn set_yaw(&self, yaw: f32) {
        self.inner.borrow_mut().set_yaw(yaw);
    }

    #[wasm_bindgen(js_name = snapYaw)]
    pub fn snap_yaw(&self, yaw: f32) {
        self.inner.borrow_mut().snap_yaw(yaw);
    }

    #[wasm_bindgen(js_name = getPitch)]
    pub fn pitch(&self) -> f32 {
        self.inner.borrow().pitch()
    }

    #[wasm_bindgen(js_name = setPitch)]
    pub fn set_pitch(&self, pitch: f32) {
        self.inner.borrow_mut().set_pitch(pitch);
    }

    #[wasm_bindgen(js_name = snapPitch)]
    pub fn snap_pitch(&self, pitch: f32) {
        self.inner.borrow_mut().snap_pitch(pitch);
    }

    #[wasm_bindgen(js_name = getRadius)]
    pub fn radius(&self) -> f32 {
        self.inner.borrow().radius()
    }

    #[wasm_bindgen(js_name = setRadius)]
    pub fn set_radius(&self, radius: f32) {
        self.inner.borrow_mut().set_radius(radius);
    }

    #[wasm_bindgen(js_name = snapRadius)]
    pub fn snap_radius(&self, radius: f32) {
        self.inner.borrow_mut().snap_radius(radius);
    }

    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(
        &self,
        position: &[f32],
        button: &str,
        pan_modifier: bool,
    ) -> Result<(), JsValue> {
        self.inner.borrow_mut().pointer_down(
            slice_to_vec2("position", position)?,
            parse_button(button)?,
            pan_modifier,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&self, position: &[f32], viewport: &[f32]) -> Result<(), JsValue> {
        self.inner.borrow_mut().pointer_move(
            slice_to_vec2("position", position)?,
            slice_to_vec2("viewport", viewport)?,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&self, button: &str) -> Result<(), JsValue> {
        self.inner.borrow_mut().pointer_up(parse_button(button)?);
        Ok(())
    }

    pub fn scroll(&self, delta: f32) {
        self.inner.borrow_mut().scroll(delta);
    }
}

impl Drop for WasmOrbitalControl {
    fn drop(&mut self) {
        self.disconnect();
    }
}

fn parse_options(
    options: Option<JsValue>,
) -> Result<(Option<web_sys::HtmlElement>, belfast::OrbitalControlOptions), JsValue> {
    let Some(options) = options.filter(|value| !value.is_undefined() && !value.is_null()) else {
        return Ok((None, belfast::OrbitalControlOptions::default()));
    };

    let object = options
        .dyn_ref::<js_sys::Object>()
        .ok_or_else(|| to_js_error("OrbitalControl options must be an object"))?;
    let options_object = js_sys::Object::assign(&js_sys::Object::new(), object);
    let listener_value =
        js_sys::Reflect::get(&options_object, &JsValue::from_str("listenerTarget"))?;
    let listener_target = if listener_value.is_undefined() || listener_value.is_null() {
        None
    } else {
        Some(
            listener_value
                .dyn_into::<web_sys::HtmlElement>()
                .map_err(|_| to_js_error("listenerTarget must be an HTMLElement"))?,
        )
    };
    js_sys::Reflect::delete_property(&options_object, &JsValue::from_str("listenerTarget"))?;

    let input: OrbitalControlOptionsInput =
        serde_wasm_bindgen::from_value(options_object.into()).map_err(to_js_error)?;
    let mut rust_options = belfast::OrbitalControlOptions::default();
    if let Some(center) = input.center {
        rust_options.center = slice_to_vec3("center", &center)?;
    }
    if let Some(radius) = input.radius {
        rust_options.radius = radius;
    }
    if let Some(min_radius) = input.min_radius {
        rust_options.min_radius = min_radius;
    }
    if let Some(max_radius) = input.max_radius {
        rust_options.max_radius = max_radius;
    }
    if let Some(rotate_sensitivity) = input.rotate_sensitivity {
        rust_options.rotate_sensitivity = rotate_sensitivity;
    }
    if let Some(zoom_sensitivity) = input.zoom_sensitivity {
        rust_options.zoom_sensitivity = zoom_sensitivity;
    }
    if let Some(pan_sensitivity) = input.pan_sensitivity {
        rust_options.pan_sensitivity = pan_sensitivity;
    }
    if let Some(damping) = input.damping {
        rust_options.damping = damping;
    }
    Ok((listener_target, rust_options))
}

fn attach_listeners(
    target: web_sys::HtmlElement,
    inner: Rc<RefCell<belfast::OrbitalControl>>,
) -> Result<AttachedListeners, JsValue> {
    let down_target = target.clone();
    let down_inner = Rc::clone(&inner);
    let on_pointer_down = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
        let Some(button) = map_pointer_button(event.button()) else {
            return;
        };
        let _ = down_target.set_pointer_capture(event.pointer_id());
        down_inner.borrow_mut().pointer_down(
            [event.client_x() as f32, event.client_y() as f32],
            button,
            event.shift_key(),
        );
    }) as Box<dyn FnMut(_)>);

    let move_target = target.clone();
    let move_inner = Rc::clone(&inner);
    let on_pointer_move = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
        move_inner.borrow_mut().pointer_move(
            [event.client_x() as f32, event.client_y() as f32],
            viewport_of(&move_target),
        );
    }) as Box<dyn FnMut(_)>);

    let up_target = target.clone();
    let up_inner = Rc::clone(&inner);
    let on_pointer_up = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
        if up_target.has_pointer_capture(event.pointer_id()) {
            let _ = up_target.release_pointer_capture(event.pointer_id());
        }
        if let Some(button) = map_pointer_button(event.button()) {
            up_inner.borrow_mut().pointer_up(button);
        }
    }) as Box<dyn FnMut(_)>);

    let wheel_inner = Rc::clone(&inner);
    let on_wheel = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
        event.prevent_default();
        wheel_inner
            .borrow_mut()
            .scroll(normalize_wheel_delta(&event));
    }) as Box<dyn FnMut(_)>);

    let event_target: &web_sys::EventTarget = target.as_ref();
    event_target.add_event_listener_with_callback(
        "pointerdown",
        on_pointer_down.as_ref().unchecked_ref(),
    )?;
    event_target.add_event_listener_with_callback(
        "pointermove",
        on_pointer_move.as_ref().unchecked_ref(),
    )?;
    event_target
        .add_event_listener_with_callback("pointerup", on_pointer_up.as_ref().unchecked_ref())?;
    event_target.add_event_listener_with_callback(
        "pointercancel",
        on_pointer_up.as_ref().unchecked_ref(),
    )?;

    let wheel_options = web_sys::AddEventListenerOptions::new();
    wheel_options.set_passive(false);
    event_target.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        on_wheel.as_ref().unchecked_ref(),
        &wheel_options,
    )?;

    Ok(AttachedListeners {
        target,
        on_pointer_down,
        on_pointer_move,
        on_pointer_up,
        on_wheel,
    })
}

fn detach_listeners(listeners: &AttachedListeners) {
    let event_target: &web_sys::EventTarget = listeners.target.as_ref();
    let _ = event_target.remove_event_listener_with_callback(
        "pointerdown",
        listeners.on_pointer_down.as_ref().unchecked_ref(),
    );
    let _ = event_target.remove_event_listener_with_callback(
        "pointermove",
        listeners.on_pointer_move.as_ref().unchecked_ref(),
    );
    let _ = event_target.remove_event_listener_with_callback(
        "pointerup",
        listeners.on_pointer_up.as_ref().unchecked_ref(),
    );
    let _ = event_target.remove_event_listener_with_callback(
        "pointercancel",
        listeners.on_pointer_up.as_ref().unchecked_ref(),
    );
    let _ = event_target
        .remove_event_listener_with_callback("wheel", listeners.on_wheel.as_ref().unchecked_ref());
}

fn viewport_of(target: &web_sys::HtmlElement) -> [f32; 2] {
    [
        target.client_width().max(0) as f32,
        target.client_height().max(0) as f32,
    ]
}

fn map_pointer_button(button: i16) -> Option<belfast::OrbitalPointerButton> {
    match button {
        0 => Some(belfast::OrbitalPointerButton::Primary),
        1 => Some(belfast::OrbitalPointerButton::Middle),
        _ => None,
    }
}

fn parse_button(button: &str) -> Result<belfast::OrbitalPointerButton, JsValue> {
    match button {
        "primary" => Ok(belfast::OrbitalPointerButton::Primary),
        "middle" => Ok(belfast::OrbitalPointerButton::Middle),
        _ => Err(to_js_error(format!(
            "unsupported orbital pointer button \"{button}\""
        ))),
    }
}

fn slice_to_vec2(name: &str, value: &[f32]) -> Result<[f32; 2], JsValue> {
    if value.len() < 2 {
        return Err(to_js_error(format!("{name} requires 2 floats")));
    }
    Ok([value[0], value[1]])
}

fn normalize_wheel_delta(event: &web_sys::WheelEvent) -> f32 {
    let delta = event.delta_y() as f32;
    match event.delta_mode() {
        1 => -delta * LINE_SCROLL_PIXELS,
        2 => -delta * PAGE_SCROLL_PIXELS,
        _ => -delta,
    }
}
