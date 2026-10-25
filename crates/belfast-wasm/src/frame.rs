use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{convert::TryFromJsValue, prelude::*, JsCast};

#[cfg(target_arch = "wasm32")]
use crate::{
    axis_helper::{AxisHelperState, WasmAxisHelper},
    ball_helper::{BallHelperState, WasmBallHelper},
    bind_group::BindGroupState,
    camera_helper::{CameraHelperState, WasmCameraHelper},
    compute::{parse_workgroups, ComputeState},
    device::{CanvasTarget, PendingGpuErrors, SurfaceLeaseGuard},
    draw::DrawState,
    texture::{TextureState, WasmTexture},
    to_js_error, WasmBindGroup, WasmCompute, WasmDraw, WasmRenderTarget,
};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
export function borrowWasmClass(value) {
    return new Proxy(value, {
        get(target, property, receiver) {
            if (property !== "__destroy_into_raw") {
                return Reflect.get(target, property, receiver);
            }
            return () => {
                const pointerDescriptor = Object.getOwnPropertyDescriptor(target, "__wbg_ptr");
                const pointer = pointerDescriptor?.value;
                if (!Number.isInteger(pointer) || pointer <= 0 || pointer > 0xffffffff) {
                    return 0;
                }

                const prototype = Object.getPrototypeOf(target);
                const cloneMethod = Object.getOwnPropertyDescriptor(
                    prototype,
                    "__frameHandle",
                )?.value;
                if (typeof cloneMethod !== "function") {
                    return 0;
                }

                const clone = Reflect.apply(cloneMethod, target, []);
                const clonePrototype = Object.getPrototypeOf(clone);
                const takeMethod = Object.getOwnPropertyDescriptor(
                    clonePrototype,
                    "__destroy_into_raw",
                )?.value;
                if (typeof takeMethod !== "function") {
                    return 0;
                }
                return Reflect.apply(takeMethod, clone, []);
            };
        },
    });
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = borrowWasmClass)]
    fn borrow_wasm_class(value: &JsValue) -> JsValue;
}

#[cfg(target_arch = "wasm32")]
std::thread_local! {
    static RENDER_TARGET_WRAPPERS: js_sys::WeakMap = js_sys::WeakMap::new();
    static BIND_GROUP_WRAPPERS: js_sys::WeakMap = js_sys::WeakMap::new();
    static SHADOW_TEXTURE_WRAPPERS: js_sys::WeakMap = js_sys::WeakMap::new();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DeviceKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct DrawKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct AxisKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct BallKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ComputeKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct BindGroupKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct CameraKey(u64);

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
enum TargetKey {
    #[default]
    Canvas,
    Offscreen(u64),
    Shadow(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DrawValidation {
    key: DrawKey,
    device: DeviceKey,
    bind_group_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CameraValidation {
    key: CameraKey,
    device: DeviceKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AxisValidation {
    key: AxisKey,
    device: DeviceKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BallValidation {
    key: BallKey,
    device: DeviceKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ComputeValidation {
    key: ComputeKey,
    device: DeviceKey,
    requires_bind_group: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindGroupValidation {
    key: BindGroupKey,
    draw: Option<DrawKey>,
    compute: Option<ComputeKey>,
    axis: Option<AxisKey>,
    ball: Option<BallKey>,
    camera: Option<CameraKey>,
    device: DeviceKey,
    group_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RenderTargetValidation {
    device: DeviceKey,
    format: wgpu::TextureFormat,
}

fn validate_render_command(
    frame_device: DeviceKey,
    draw: DrawValidation,
    bind_groups: &[BindGroupValidation],
) -> Result<(), String> {
    if draw.device != frame_device {
        return Err("draw was created by a different device".into());
    }

    if bind_groups.len() != draw.bind_group_count {
        return Err(match draw.bind_group_count {
            0 => "draw does not accept a bind group".into(),
            1 => "draw requires a bind group".into(),
            count => format!("draw requires {count} bind groups"),
        });
    }

    for bind_group in bind_groups {
        if bind_group.device != frame_device {
            return Err("bind group was created by a different device".into());
        }
        if bind_group.draw != Some(draw.key) {
            return Err("bind group was created for a different draw".into());
        }
    }

    if draw.bind_group_count == 2 {
        let groups: Vec<_> = bind_groups.iter().map(|group| group.group_index).collect();
        if !(groups.contains(&0) && groups.contains(&1)) {
            return Err("draw requires bind groups for @group(0) and @group(1)".into());
        }
    }
    if draw.bind_group_count >= 3
        && !(0..draw.bind_group_count as u32)
            .all(|index| bind_groups.iter().any(|group| group.group_index == index))
    {
        return Err(format!(
            "draw requires every bind group from @group(0) through @group({})",
            draw.bind_group_count - 1
        ));
    }

    Ok(())
}

fn validate_camera_render_command(
    frame_device: DeviceKey,
    camera: CameraValidation,
    bind_group: Option<BindGroupValidation>,
) -> Result<(), String> {
    if camera.device != frame_device {
        return Err("camera helper was created by a different device".into());
    }
    let Some(bind_group) = bind_group else {
        return Err("camera helper requires a bind group".into());
    };
    if bind_group.device != frame_device {
        return Err("bind group was created by a different device".into());
    }
    if bind_group.camera != Some(camera.key) {
        return Err("bind group was created for a different camera helper".into());
    }
    Ok(())
}

fn validate_axis_render_command(
    frame_device: DeviceKey,
    axis: AxisValidation,
    bind_group: Option<BindGroupValidation>,
) -> Result<(), String> {
    if axis.device != frame_device {
        return Err("axis helper was created by a different device".into());
    }

    let Some(bind_group) = bind_group else {
        return Err("axis helper requires a bind group".into());
    };
    if bind_group.device != frame_device {
        return Err("bind group was created by a different device".into());
    }
    if bind_group.axis != Some(axis.key) {
        return Err("bind group was created for a different axis helper".into());
    }
    Ok(())
}

fn validate_ball_render_command(
    frame_device: DeviceKey,
    ball: BallValidation,
    bind_group: Option<BindGroupValidation>,
) -> Result<(), String> {
    if ball.device != frame_device {
        return Err("ball helper was created by a different device".into());
    }

    let Some(bind_group) = bind_group else {
        return Err("ball helper requires a bind group".into());
    };
    if bind_group.device != frame_device {
        return Err("bind group was created by a different device".into());
    }
    if bind_group.ball != Some(ball.key) {
        return Err("bind group was created for a different ball helper".into());
    }
    Ok(())
}

fn validate_dispatch_command(
    frame_device: DeviceKey,
    compute: ComputeValidation,
    bind_group: Option<BindGroupValidation>,
) -> Result<(), String> {
    if compute.device != frame_device {
        return Err("compute was created by a different device".into());
    }

    match (compute.requires_bind_group, bind_group) {
        (true, None) => Err("compute requires a bind group".into()),
        (false, Some(_)) => Err("compute does not accept a bind group".into()),
        (false, None) => Ok(()),
        (true, Some(bind_group)) => {
            if bind_group.device != frame_device {
                return Err("bind group was created by a different device".into());
            }
            if bind_group.compute != Some(compute.key) {
                return Err("bind group was created for a different compute".into());
            }
            Ok(())
        }
    }
}

fn validate_render_target(
    frame_device: DeviceKey,
    frame_format: wgpu::TextureFormat,
    target: RenderTargetValidation,
) -> Result<(), String> {
    if target.device != frame_device {
        return Err("render target was created by a different device".into());
    }
    if target.format != frame_format {
        return Err("render target format does not match frame format".into());
    }
    Ok(())
}

fn validate_clear_color(color: wgpu::Color) -> Result<wgpu::Color, String> {
    if [color.r, color.g, color.b, color.a]
        .into_iter()
        .all(f64::is_finite)
    {
        Ok(color)
    } else {
        Err("clearColor components must be finite".into())
    }
}

fn validate_live_wrapper_pointer(pointer: Option<f64>) -> Result<u32, String> {
    let Some(pointer) = pointer else {
        return Err("wrapper must contain a live wasm pointer".into());
    };
    if !pointer.is_finite()
        || pointer.fract() != 0.0
        || pointer < 1.0
        || pointer > f64::from(u32::MAX)
    {
        return Err("wrapper must contain a live wasm pointer".into());
    }
    Ok(pointer as u32)
}

fn validate_registered_wrapper_pointer(
    current: Option<f64>,
    registered: Option<f64>,
) -> Result<u32, String> {
    const ERROR: &str = "wrapper must be registered with its original live wasm pointer";

    let current = validate_live_wrapper_pointer(current).map_err(|_| ERROR.to_owned())?;
    let registered = validate_live_wrapper_pointer(registered).map_err(|_| ERROR.to_owned())?;
    if current != registered {
        return Err(ERROR.into());
    }
    Ok(current)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlannedLoadOp {
    Clear,
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlannedDrawable {
    Draw(DrawKey),
    Axis(AxisKey),
    Ball(BallKey),
    Camera(CameraKey),
}

#[derive(Debug, PartialEq)]
struct PlannedRenderCommand {
    drawable: PlannedDrawable,
    bind_groups: Vec<BindGroupKey>,
    instance_count: u32,
}

#[derive(Debug, PartialEq)]
struct PlannedRenderPass {
    target: TargetKey,
    load_op: PlannedLoadOp,
    clear_color: wgpu::Color,
    depth: bool,
    commands: Vec<PlannedRenderCommand>,
}

#[derive(Debug, PartialEq)]
enum PlannedOp {
    Compute {
        compute: ComputeKey,
        bind_group: Option<BindGroupKey>,
        workgroups: [u32; 3],
    },
    RenderPass(PlannedRenderPass),
}

#[derive(Debug, PartialEq)]
enum RecordedCommand {
    BindTarget {
        target: TargetKey,
        clear_color: wgpu::Color,
        depth: bool,
    },
    Render {
        drawable: PlannedDrawable,
        bind_groups: Vec<BindGroupKey>,
        instance_count: u32,
    },
    Dispatch {
        compute: ComputeKey,
        bind_group: Option<BindGroupKey>,
        workgroups: [u32; 3],
    },
}

struct FramePlan {
    commands: Vec<RecordedCommand>,
}

impl FramePlan {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    fn bind_target(&mut self, target: TargetKey, clear_color: wgpu::Color, depth: bool) {
        self.commands.push(RecordedCommand::BindTarget {
            target,
            clear_color,
            depth,
        });
    }

    fn render(
        &mut self,
        draw: DrawKey,
        bind_groups: Vec<BindGroupKey>,
        instance_count: u32,
    ) -> Result<(), String> {
        self.commands.push(RecordedCommand::Render {
            drawable: PlannedDrawable::Draw(draw),
            bind_groups,
            instance_count,
        });
        Ok(())
    }

    fn render_axis(&mut self, axis: AxisKey, bind_group: BindGroupKey) -> Result<(), String> {
        self.commands.push(RecordedCommand::Render {
            drawable: PlannedDrawable::Axis(axis),
            bind_groups: vec![bind_group],
            instance_count: 1,
        });
        Ok(())
    }

    fn render_ball(&mut self, ball: BallKey, bind_group: BindGroupKey) -> Result<(), String> {
        self.commands.push(RecordedCommand::Render {
            drawable: PlannedDrawable::Ball(ball),
            bind_groups: vec![bind_group],
            instance_count: 1,
        });
        Ok(())
    }

    fn render_camera(&mut self, camera: CameraKey, bind_group: BindGroupKey) -> Result<(), String> {
        self.commands.push(RecordedCommand::Render {
            drawable: PlannedDrawable::Camera(camera),
            bind_groups: vec![bind_group],
            instance_count: 1,
        });
        Ok(())
    }

    fn dispatch(
        &mut self,
        compute: ComputeKey,
        bind_group: Option<BindGroupKey>,
        workgroups: [u32; 3],
    ) -> Result<(), String> {
        self.commands.push(RecordedCommand::Dispatch {
            compute,
            bind_group,
            workgroups,
        });
        Ok(())
    }

    fn finish(self) -> Result<Vec<PlannedOp>, String> {
        let mut ops = Vec::new();
        let mut current_pass: Option<PlannedRenderPass> = None;
        let mut bound_target: Option<(TargetKey, wgpu::Color, bool)> = None;
        let mut next_load = HashMap::new();

        let flush_pass =
            |current_pass: &mut Option<PlannedRenderPass>,
             ops: &mut Vec<PlannedOp>,
             next_load: &mut HashMap<TargetKey, PlannedLoadOp>| {
                if let Some(pass) = current_pass.take() {
                    if !pass.commands.is_empty() {
                        next_load.insert(pass.target, PlannedLoadOp::Load);
                        ops.push(PlannedOp::RenderPass(pass));
                    }
                }
            };

        for command in self.commands {
            match command {
                RecordedCommand::BindTarget {
                    target,
                    clear_color,
                    depth,
                } => {
                    flush_pass(&mut current_pass, &mut ops, &mut next_load);
                    bound_target = Some((target, clear_color, depth));
                    next_load.insert(target, PlannedLoadOp::Clear);
                }
                RecordedCommand::Render {
                    drawable,
                    bind_groups,
                    instance_count,
                } => {
                    let (target, clear_color, depth) = bound_target
                        .unwrap_or_else(|| (TargetKey::Canvas, default_clear_color(), false));
                    if bound_target.is_none() {
                        bound_target = Some((target, clear_color, depth));
                    }
                    let needs_new_pass = current_pass
                        .as_ref()
                        .is_none_or(|pass| pass.target != target);
                    if needs_new_pass {
                        flush_pass(&mut current_pass, &mut ops, &mut next_load);
                        current_pass = Some(PlannedRenderPass {
                            target,
                            load_op: next_load
                                .get(&target)
                                .copied()
                                .unwrap_or(PlannedLoadOp::Clear),
                            clear_color,
                            depth,
                            commands: Vec::new(),
                        });
                    }
                    current_pass
                        .as_mut()
                        .expect("frame plan has a current pass")
                        .commands
                        .push(PlannedRenderCommand {
                            drawable,
                            bind_groups,
                            instance_count,
                        });
                }
                RecordedCommand::Dispatch {
                    compute,
                    bind_group,
                    workgroups,
                } => {
                    flush_pass(&mut current_pass, &mut ops, &mut next_load);
                    ops.push(PlannedOp::Compute {
                        compute,
                        bind_group,
                        workgroups,
                    });
                }
            }
        }
        flush_pass(&mut current_pass, &mut ops, &mut next_load);
        if ops.is_empty() {
            return Err("frame contains no commands".into());
        }
        Ok(ops)
    }
}

fn default_clear_color() -> wgpu::Color {
    wgpu::Color {
        r: 0.02,
        g: 0.025,
        b: 0.04,
        a: 1.0,
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FramePassOptionsInput {
    #[serde(default)]
    clear_color: Option<ClearColorInput>,
    #[serde(default)]
    depth: bool,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ClearColorInput {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

#[cfg(target_arch = "wasm32")]
impl From<ClearColorInput> for wgpu::Color {
    fn from(value: ClearColorInput) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_pass_options(options: JsValue) -> Result<(wgpu::Color, bool), JsValue> {
    let options = if options.is_undefined() {
        FramePassOptionsInput::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(to_js_error)?
    };
    let clear_color = validate_clear_color(
        options
            .clear_color
            .map(Into::into)
            .unwrap_or_else(default_clear_color),
    )
    .map_err(to_js_error)?;
    Ok((clear_color, options.depth))
}

#[cfg(target_arch = "wasm32")]
fn optional_bind_group(value: JsValue) -> Result<Option<WasmBindGroup>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    let handle = borrow_registered_wasm_class(
        &value,
        &BIND_GROUP_WRAPPERS,
        "bindGroup must be a live BindGroup or null",
    )?;
    WasmBindGroup::try_from_js_value(handle)
        .map(Some)
        .map_err(|_| to_js_error("bindGroup must be a live BindGroup or null"))
}

#[cfg(target_arch = "wasm32")]
fn wrapper_object_and_pointer<'a>(
    value: &'a JsValue,
    error: &str,
) -> Result<(&'a js_sys::Object, Option<f64>), JsValue> {
    let object = value
        .dyn_ref::<js_sys::Object>()
        .ok_or_else(|| to_js_error(error))?;
    let descriptor =
        js_sys::Reflect::get_own_property_descriptor(object, &JsValue::from_str("__wbg_ptr"))
            .map_err(|_| to_js_error(error))?;
    let pointer = if descriptor.is_undefined() {
        None
    } else {
        js_sys::Reflect::get(&descriptor, &JsValue::from_str("value"))
            .ok()
            .and_then(|value| value.as_f64())
    };
    Ok((object, pointer))
}

#[cfg(target_arch = "wasm32")]
fn register_wasm_class(
    value: &JsValue,
    wrappers: &'static std::thread::LocalKey<js_sys::WeakMap>,
    error: &str,
) -> Result<(), JsValue> {
    let (object, pointer) = wrapper_object_and_pointer(value, error)?;
    let pointer = validate_live_wrapper_pointer(pointer).map_err(|_| to_js_error(error))?;
    wrappers.with(|wrappers| {
        wrappers.set(object, &JsValue::from_f64(f64::from(pointer)));
    });
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn borrow_registered_wasm_class(
    value: &JsValue,
    wrappers: &'static std::thread::LocalKey<js_sys::WeakMap>,
    error: &str,
) -> Result<JsValue, JsValue> {
    let (object, current_pointer) = wrapper_object_and_pointer(value, error)?;
    let registered_pointer = wrappers
        .with(|wrappers| wrappers.get_checked(object))
        .and_then(|pointer| pointer.as_f64());
    validate_registered_wrapper_pointer(current_pointer, registered_pointer)
        .map_err(|_| to_js_error(error))?;

    // Exact object identity and the original pointer are checked before the
    // generated lexical instanceof check can clone or enter a Rust &self method.
    Ok(borrow_wasm_class(value))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn register_render_target_wrapper(value: &JsValue) -> Result<(), JsValue> {
    register_wasm_class(
        value,
        &RENDER_TARGET_WRAPPERS,
        "failed to register RenderTarget wrapper",
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn register_shadow_texture_wrapper(value: &JsValue) -> Result<(), JsValue> {
    register_wasm_class(
        value,
        &SHADOW_TEXTURE_WRAPPERS,
        "failed to register shadow Texture wrapper",
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn register_bind_group_wrapper(value: &JsValue) -> Result<(), JsValue> {
    register_wasm_class(
        value,
        &BIND_GROUP_WRAPPERS,
        "failed to register BindGroup wrapper",
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn clone_live_wasm_class(value: &JsValue, error: &str) -> Result<JsValue, JsValue> {
    let (_object, pointer) = wrapper_object_and_pointer(value, error)?;
    validate_live_wrapper_pointer(pointer).map_err(|_| to_js_error(error))?;
    Ok(borrow_wasm_class(value))
}

#[cfg(target_arch = "wasm32")]
pub(crate) enum Drawable {
    Draw(WasmDraw),
    Axis(WasmAxisHelper),
    Ball(WasmBallHelper),
    Camera(WasmCameraHelper),
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn parse_drawable(value: &JsValue) -> Result<Drawable, JsValue> {
    const ERROR: &str = "value must be a live Draw, AxisHelper, BallHelper, or CameraHelper";
    let draw_handle = clone_live_wasm_class(value, ERROR)?;
    if let Ok(draw) = WasmDraw::try_from_js_value(draw_handle) {
        return Ok(Drawable::Draw(draw));
    }
    let axis_handle = clone_live_wasm_class(value, ERROR)?;
    if let Ok(axes) = WasmAxisHelper::try_from_js_value(axis_handle) {
        return Ok(Drawable::Axis(axes));
    }
    let ball_handle = clone_live_wasm_class(value, ERROR)?;
    if let Ok(ball) = WasmBallHelper::try_from_js_value(ball_handle) {
        return Ok(Drawable::Ball(ball));
    }
    let camera_handle = clone_live_wasm_class(value, ERROR)?;
    WasmCameraHelper::try_from_js_value(camera_handle)
        .map(Drawable::Camera)
        .map_err(|_| to_js_error(ERROR))
}

#[cfg(target_arch = "wasm32")]
enum ParsedFrameTarget {
    Canvas,
    RenderTarget(WasmRenderTarget),
    ShadowMap(WasmTexture),
}

#[cfg(target_arch = "wasm32")]
fn parse_frame_target(value: JsValue) -> Result<ParsedFrameTarget, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(ParsedFrameTarget::Canvas);
    }
    if let Ok(handle) = borrow_registered_wasm_class(
        &value,
        &SHADOW_TEXTURE_WRAPPERS,
        "target must be a live RenderTarget, Texture, or null",
    ) {
        if let Ok(texture) = WasmTexture::try_from_js_value(handle) {
            return Ok(ParsedFrameTarget::ShadowMap(texture));
        }
    }
    let handle = borrow_registered_wasm_class(
        &value,
        &RENDER_TARGET_WRAPPERS,
        "target must be a live RenderTarget, Texture, or null",
    )?;
    WasmRenderTarget::try_from_js_value(handle)
        .map(ParsedFrameTarget::RenderTarget)
        .map_err(|_| to_js_error("target must be a live RenderTarget, Texture, or null"))
}

#[cfg(target_arch = "wasm32")]
fn parse_bind_groups(value: JsValue) -> Result<Vec<WasmBindGroup>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(Vec::new());
    }
    if js_sys::Array::is_array(&value) {
        let array = js_sys::Array::from(&value);
        let mut groups = Vec::with_capacity(array.length() as usize);
        for index in 0..array.length() {
            let entry = array.get(index);
            groups.push(required_bind_group(entry)?);
        }
        return Ok(groups);
    }
    Ok(vec![required_bind_group(value)?])
}

#[cfg(target_arch = "wasm32")]
fn required_bind_group(value: JsValue) -> Result<WasmBindGroup, JsValue> {
    optional_bind_group(value)?
        .ok_or_else(|| to_js_error("bindGroup must be a live BindGroup or null"))
}

#[cfg(target_arch = "wasm32")]
enum FrameTarget {
    Canvas,
    RenderTarget(Rc<RefCell<belfast::RenderTarget>>),
    ShadowMap(Rc<TextureState>),
}

#[cfg(target_arch = "wasm32")]
enum RenderCommand {
    Draw {
        draw: Rc<DrawState>,
        bind_groups: Vec<Rc<BindGroupState>>,
        instance_count: u32,
    },
    Axis {
        axis: Rc<AxisHelperState>,
        bind_group: Rc<BindGroupState>,
    },
    Ball {
        ball: Rc<BallHelperState>,
        bind_group: Rc<BindGroupState>,
    },
    Camera {
        camera: Rc<CameraHelperState>,
        bind_group: Rc<BindGroupState>,
    },
}

#[cfg(target_arch = "wasm32")]
struct ComputeCommand {
    compute: Rc<ComputeState>,
    bind_group: Option<Rc<BindGroupState>>,
    workgroups: [u32; 3],
}

#[cfg(target_arch = "wasm32")]
struct LogicalPass {
    target: FrameTarget,
    load_op: PlannedLoadOp,
    clear_color: wgpu::Color,
    depth: bool,
    commands: Vec<RenderCommand>,
}

#[cfg(target_arch = "wasm32")]
enum LogicalOp {
    Compute(ComputeCommand),
    Render(LogicalPass),
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(typescript_custom_section)]
const FRAME_PASS_TYPES: &str = r#"
export interface FramePassOptions {
  clearColor?: { r: number; g: number; b: number; a: number };
  depth?: boolean;
}
"#;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = Frame)]
pub struct WasmFrame {
    device: belfast::Device,
    pending_gpu_errors: PendingGpuErrors,
    canvas_target: Rc<RefCell<CanvasTarget>>,
    surface_texture: Option<wgpu::SurfaceTexture>,
    surface_lease: SurfaceLeaseGuard,
    canvas_view: wgpu::TextureView,
    reconfigure_after_present: bool,
    plan: FramePlan,
    render_targets: HashMap<TargetKey, Rc<RefCell<belfast::RenderTarget>>>,
    shadow_maps: HashMap<TargetKey, Rc<TextureState>>,
    draws: HashMap<DrawKey, Rc<DrawState>>,
    axes: HashMap<AxisKey, Rc<AxisHelperState>>,
    balls: HashMap<BallKey, Rc<BallHelperState>>,
    cameras: HashMap<CameraKey, Rc<CameraHelperState>>,
    computes: HashMap<ComputeKey, Rc<ComputeState>>,
    bind_groups: HashMap<BindGroupKey, Rc<BindGroupState>>,
}

#[cfg(target_arch = "wasm32")]
impl WasmFrame {
    pub(crate) fn new(
        device: belfast::Device,
        pending_gpu_errors: PendingGpuErrors,
        canvas_target: Rc<RefCell<CanvasTarget>>,
        surface_texture: wgpu::SurfaceTexture,
        surface_lease: SurfaceLeaseGuard,
        reconfigure_after_present: bool,
    ) -> Self {
        let canvas_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            device,
            pending_gpu_errors,
            canvas_target,
            surface_texture: Some(surface_texture),
            surface_lease,
            canvas_view,
            reconfigure_after_present,
            plan: FramePlan::new(),
            render_targets: HashMap::new(),
            shadow_maps: HashMap::new(),
            draws: HashMap::new(),
            axes: HashMap::new(),
            balls: HashMap::new(),
            cameras: HashMap::new(),
            computes: HashMap::new(),
            bind_groups: HashMap::new(),
        }
    }

    pub(crate) fn render_draw(
        &mut self,
        draw: &WasmDraw,
        bind_groups: Vec<WasmBindGroup>,
        instance_count: u32,
    ) -> Result<(), JsValue> {
        let draw_key = DrawKey(rc_key(&draw.state));
        let bind_group_validations: Vec<_> =
            bind_groups.iter().map(bind_group_validation).collect();
        validate_render_command(
            device_key(&self.device),
            DrawValidation {
                key: draw_key,
                device: device_key(draw.state.draw().device()),
                bind_group_count: draw.state.resources.bind_group_count(),
            },
            &bind_group_validations,
        )
        .map_err(to_js_error)?;

        {
            let mesh = draw.state.mesh();
            draw.state
                .draw()
                .validate_for_render(&self.device, &mesh)
                .map_err(to_js_error)?;
        }

        self.draws.insert(draw_key, draw.state.clone());
        let mut bind_group_keys = Vec::with_capacity(bind_groups.len());
        for bind_group in bind_groups {
            let key = BindGroupKey(rc_key(&bind_group.state));
            self.bind_groups.insert(key, bind_group.state.clone());
            bind_group_keys.push(key);
        }
        self.plan
            .render(draw_key, bind_group_keys, instance_count)
            .map_err(to_js_error)
    }

    fn render_axis(&mut self, axes: &WasmAxisHelper, bind_group: JsValue) -> Result<(), JsValue> {
        let bind_group_handle = optional_bind_group(bind_group)?;
        let bind_group = bind_group_handle.as_ref();
        let axis_key = AxisKey(rc_key(&axes.state));
        let bind_group_validation = bind_group.map(bind_group_validation);
        validate_axis_render_command(
            device_key(&self.device),
            AxisValidation {
                key: axis_key,
                device: device_key(axes.state.helper().device()),
            },
            bind_group_validation,
        )
        .map_err(to_js_error)?;

        self.axes.insert(axis_key, axes.state.clone());
        let bind_group =
            bind_group.ok_or_else(|| to_js_error("axis helper requires a bind group"))?;
        self.bind_groups.insert(
            BindGroupKey(rc_key(&bind_group.state)),
            bind_group.state.clone(),
        );
        self.plan
            .render_axis(axis_key, BindGroupKey(rc_key(&bind_group.state)))
            .map_err(to_js_error)
    }

    fn render_ball(&mut self, ball: &WasmBallHelper, bind_group: JsValue) -> Result<(), JsValue> {
        let bind_group_handle = optional_bind_group(bind_group)?;
        let bind_group = bind_group_handle.as_ref();
        let ball_key = BallKey(rc_key(&ball.state));
        let bind_group_validation = bind_group.map(bind_group_validation);
        validate_ball_render_command(
            device_key(&self.device),
            BallValidation {
                key: ball_key,
                device: device_key(ball.state.helper().device()),
            },
            bind_group_validation,
        )
        .map_err(to_js_error)?;

        self.balls.insert(ball_key, ball.state.clone());
        let bind_group =
            bind_group.ok_or_else(|| to_js_error("ball helper requires a bind group"))?;
        self.bind_groups.insert(
            BindGroupKey(rc_key(&bind_group.state)),
            bind_group.state.clone(),
        );
        self.plan
            .render_ball(ball_key, BindGroupKey(rc_key(&bind_group.state)))
            .map_err(to_js_error)
    }

    fn render_camera(
        &mut self,
        camera: &WasmCameraHelper,
        bind_group: JsValue,
    ) -> Result<(), JsValue> {
        let bind_group_handle = optional_bind_group(bind_group)?;
        let bind_group = bind_group_handle.as_ref();
        let camera_key = CameraKey(rc_key(&camera.state));
        let bind_group_validation = bind_group.map(bind_group_validation);
        validate_camera_render_command(
            device_key(&self.device),
            CameraValidation {
                key: camera_key,
                device: device_key(camera.state.helper().device()),
            },
            bind_group_validation,
        )
        .map_err(to_js_error)?;

        self.cameras.insert(camera_key, camera.state.clone());
        let bind_group =
            bind_group.ok_or_else(|| to_js_error("camera helper requires a bind group"))?;
        self.bind_groups.insert(
            BindGroupKey(rc_key(&bind_group.state)),
            bind_group.state.clone(),
        );
        self.plan
            .render_camera(camera_key, BindGroupKey(rc_key(&bind_group.state)))
            .map_err(to_js_error)
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WasmFrame {
    fn drop(&mut self) {
        drop(self.surface_texture.take());
        self.surface_lease.release();
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = Frame)]
impl WasmFrame {
    #[wasm_bindgen(js_name = bindTarget)]
    pub fn bind_target(
        &mut self,
        #[wasm_bindgen(unchecked_optional_param_type = "RenderTarget | Texture | null")]
        target: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "FramePassOptions")] options: JsValue,
    ) -> Result<(), JsValue> {
        let (clear_color, mut depth) = parse_pass_options(options)?;
        let target = match parse_frame_target(target)? {
            ParsedFrameTarget::Canvas => TargetKey::Canvas,
            ParsedFrameTarget::RenderTarget(target) => {
                let target_key = TargetKey::Offscreen(rc_key(&target.target));
                let target_format = target.target.borrow().format();
                validate_render_target(
                    device_key(&self.device),
                    self.device.format(),
                    RenderTargetValidation {
                        device: device_key(&target.device),
                        format: target_format,
                    },
                )
                .map_err(to_js_error)?;
                self.render_targets
                    .insert(target_key, target.target.clone());
                target_key
            }
            ParsedFrameTarget::ShadowMap(texture) => {
                if !texture.state.is_shadow_map() {
                    return Err(to_js_error("texture must be a depth shadow map"));
                }
                if !texture.state.texture().device().is_same(&self.device) {
                    return Err(to_js_error("shadow map was created by a different device"));
                }
                let target_key = TargetKey::Shadow(rc_key(&texture.state));
                self.shadow_maps.insert(target_key, texture.state.clone());
                target_key
            }
        };
        depth = match target {
            TargetKey::Canvas => depth,
            TargetKey::Offscreen(_) => false,
            TargetKey::Shadow(_) => true,
        };
        self.plan.bind_target(target, clear_color, depth);
        Ok(())
    }

    pub fn render(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Draw | AxisHelper | BallHelper | CameraHelper")]
        drawable: JsValue,
        #[wasm_bindgen(
            js_name = bindGroup,
            unchecked_optional_param_type = "BindGroup | BindGroup[] | null"
        )]
        bind_group: JsValue,
        #[wasm_bindgen(js_name = instanceCount, unchecked_optional_param_type = "number")]
        instance_count: JsValue,
    ) -> Result<(), JsValue> {
        let has_instance_count = !instance_count.is_undefined() && !instance_count.is_null();
        match parse_drawable(&drawable)? {
            Drawable::Draw(draw) => {
                let instance_count = if has_instance_count {
                    Some(instance_count.as_f64().ok_or_else(|| {
                        to_js_error(
                            "instanceCount must be a finite integer between 0 and 4294967295",
                        )
                    })?)
                } else {
                    None
                };
                let instance_count =
                    crate::bindings::parse_instance_count(instance_count).map_err(to_js_error)?;
                let bind_groups = parse_bind_groups(bind_group)?;
                self.render_draw(&draw, bind_groups, instance_count)
            }
            Drawable::Axis(axes) => {
                if has_instance_count {
                    return Err(to_js_error(
                        "instanceCount is only supported when rendering a Draw",
                    ));
                }
                self.render_axis(&axes, bind_group)
            }
            Drawable::Ball(ball) => {
                if has_instance_count {
                    return Err(to_js_error(
                        "instanceCount is only supported when rendering a Draw",
                    ));
                }
                self.render_ball(&ball, bind_group)
            }
            Drawable::Camera(camera) => {
                if has_instance_count {
                    return Err(to_js_error(
                        "instanceCount is only supported when rendering a Draw",
                    ));
                }
                self.render_camera(&camera, bind_group)
            }
        }
    }

    pub fn dispatch(
        &mut self,
        compute: &WasmCompute,
        #[wasm_bindgen(
            js_name = bindGroup,
            unchecked_optional_param_type = "BindGroup | null"
        )]
        bind_group: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "WorkgroupCount")] workgroups: JsValue,
    ) -> Result<(), JsValue> {
        let bind_group_handle = optional_bind_group(bind_group)?;
        let bind_group = bind_group_handle.as_ref();
        let compute_key = ComputeKey(rc_key(&compute.state));
        let bind_group_validation = bind_group.map(bind_group_validation);
        let workgroups = parse_workgroups(
            &workgroups,
            self.device
                .gpu()
                .limits()
                .max_compute_workgroups_per_dimension,
        )
        .map_err(to_js_error)?;
        validate_dispatch_command(
            device_key(&self.device),
            ComputeValidation {
                key: compute_key,
                device: device_key(compute.state.compute().device()),
                requires_bind_group: compute.state.requires_bind_group(),
            },
            bind_group_validation,
        )
        .map_err(to_js_error)?;
        compute
            .state
            .compute()
            .validate_for_dispatch(&self.device)
            .map_err(to_js_error)?;

        self.computes.insert(compute_key, compute.state.clone());
        if let Some(bind_group) = bind_group {
            self.bind_groups.insert(
                BindGroupKey(rc_key(&bind_group.state)),
                bind_group.state.clone(),
            );
        }
        self.plan
            .dispatch(
                compute_key,
                bind_group_validation.map(|binding| binding.key),
                workgroups,
            )
            .map_err(to_js_error)
    }

    pub fn submit(mut self) -> Result<(), JsValue> {
        let plan = std::mem::replace(&mut self.plan, FramePlan::new());
        let planned_ops = plan.finish().map_err(to_js_error)?;
        let logical_ops = materialize_ops(
            planned_ops,
            &self.render_targets,
            &self.shadow_maps,
            &self.draws,
            &self.axes,
            &self.balls,
            &self.cameras,
            &self.computes,
            &self.bind_groups,
        )
        .map_err(to_js_error)?;
        let mut encoder =
            self.device
                .gpu()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("BelfastFrameEncoder"),
                });

        let needs_canvas_depth = logical_ops.iter().any(|op| match op {
            LogicalOp::Render(pass) => matches!(pass.target, FrameTarget::Canvas) && pass.depth,
            LogicalOp::Compute(_) => false,
        });
        if needs_canvas_depth {
            let (width, height) = self.canvas_target.borrow().size();
            self.canvas_target
                .borrow_mut()
                .ensure_depth(self.device.gpu(), width, height)
                .map_err(to_js_error)?;
        }

        for logical_op in logical_ops {
            encode_logical_op(
                &mut encoder,
                &self.canvas_view,
                &self.canvas_target,
                logical_op,
            );
        }

        self.device.queue().submit([encoder.finish()]);
        let surface_texture = self
            .surface_texture
            .take()
            .ok_or_else(|| to_js_error("frame surface texture is unavailable"))?;
        self.device.queue().present(surface_texture);
        self.surface_lease.release();
        if self.reconfigure_after_present {
            self.canvas_target.borrow().configure(self.device.gpu());
        }
        if let Some(error) = self.pending_gpu_errors.take() {
            return Err(to_js_error(error));
        }
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
fn bind_group_validation(bind_group: &WasmBindGroup) -> BindGroupValidation {
    BindGroupValidation {
        key: BindGroupKey(rc_key(&bind_group.state)),
        draw: bind_group.state.draw().map(|draw| DrawKey(rc_key(draw))),
        compute: bind_group
            .state
            .compute()
            .map(|compute| ComputeKey(rc_key(compute))),
        axis: bind_group
            .state
            .axis_helper()
            .map(|axis| AxisKey(rc_key(axis))),
        ball: bind_group
            .state
            .ball_helper()
            .map(|ball| BallKey(rc_key(ball))),
        camera: bind_group
            .state
            .camera_helper()
            .map(|camera| CameraKey(rc_key(camera))),
        device: device_key(bind_group.state.bind_group().device()),
        group_index: bind_group.state.group_index(),
    }
}

#[cfg(target_arch = "wasm32")]
fn depth_load_op(load_op: PlannedLoadOp) -> wgpu::LoadOp<f32> {
    match load_op {
        PlannedLoadOp::Clear => wgpu::LoadOp::Clear(1.0),
        PlannedLoadOp::Load => wgpu::LoadOp::Load,
    }
}

#[cfg(target_arch = "wasm32")]
fn color_load_op(load_op: PlannedLoadOp, clear_color: wgpu::Color) -> wgpu::LoadOp<wgpu::Color> {
    match load_op {
        PlannedLoadOp::Clear => wgpu::LoadOp::Clear(clear_color),
        PlannedLoadOp::Load => wgpu::LoadOp::Load,
    }
}

#[cfg(target_arch = "wasm32")]
fn materialize_ops(
    planned_ops: Vec<PlannedOp>,
    render_targets: &HashMap<TargetKey, Rc<RefCell<belfast::RenderTarget>>>,
    shadow_maps: &HashMap<TargetKey, Rc<TextureState>>,
    draws: &HashMap<DrawKey, Rc<DrawState>>,
    axes: &HashMap<AxisKey, Rc<AxisHelperState>>,
    balls: &HashMap<BallKey, Rc<BallHelperState>>,
    cameras: &HashMap<CameraKey, Rc<CameraHelperState>>,
    computes: &HashMap<ComputeKey, Rc<ComputeState>>,
    bind_groups: &HashMap<BindGroupKey, Rc<BindGroupState>>,
) -> Result<Vec<LogicalOp>, String> {
    planned_ops
        .into_iter()
        .map(|planned_op| match planned_op {
            PlannedOp::Compute {
                compute,
                bind_group,
                workgroups,
            } => Ok(LogicalOp::Compute(ComputeCommand {
                compute: computes
                    .get(&compute)
                    .cloned()
                    .ok_or_else(|| "frame compute is unavailable".to_owned())?,
                bind_group: bind_group
                    .map(|key| {
                        bind_groups
                            .get(&key)
                            .cloned()
                            .ok_or_else(|| "frame bind group is unavailable".to_owned())
                    })
                    .transpose()?,
                workgroups,
            })),
            PlannedOp::RenderPass(planned_pass) => {
                let target = match planned_pass.target {
                    TargetKey::Canvas => FrameTarget::Canvas,
                    key @ TargetKey::Offscreen(_) => FrameTarget::RenderTarget(
                        render_targets
                            .get(&key)
                            .cloned()
                            .ok_or_else(|| "frame render target is unavailable".to_owned())?,
                    ),
                    key @ TargetKey::Shadow(_) => FrameTarget::ShadowMap(
                        shadow_maps
                            .get(&key)
                            .cloned()
                            .ok_or_else(|| "frame shadow map is unavailable".to_owned())?,
                    ),
                };
                let commands = planned_pass
                    .commands
                    .into_iter()
                    .map(|command| {
                        let resolved_bind_groups = command
                            .bind_groups
                            .into_iter()
                            .map(|key| {
                                bind_groups
                                    .get(&key)
                                    .cloned()
                                    .ok_or_else(|| "frame bind group is unavailable".to_owned())
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        match command.drawable {
                            PlannedDrawable::Draw(draw) => Ok(RenderCommand::Draw {
                                draw: draws
                                    .get(&draw)
                                    .cloned()
                                    .ok_or_else(|| "frame draw is unavailable".to_owned())?,
                                bind_groups: resolved_bind_groups,
                                instance_count: command.instance_count,
                            }),
                            PlannedDrawable::Axis(axis) => Ok(RenderCommand::Axis {
                                axis: axes
                                    .get(&axis)
                                    .cloned()
                                    .ok_or_else(|| "frame axis helper is unavailable".to_owned())?,
                                bind_group: resolved_bind_groups.into_iter().next().ok_or_else(
                                    || "axis helper requires a bind group".to_owned(),
                                )?,
                            }),
                            PlannedDrawable::Ball(ball) => Ok(RenderCommand::Ball {
                                ball: balls
                                    .get(&ball)
                                    .cloned()
                                    .ok_or_else(|| "frame ball helper is unavailable".to_owned())?,
                                bind_group: resolved_bind_groups.into_iter().next().ok_or_else(
                                    || "ball helper requires a bind group".to_owned(),
                                )?,
                            }),
                            PlannedDrawable::Camera(camera) => Ok(RenderCommand::Camera {
                                camera: cameras.get(&camera).cloned().ok_or_else(|| {
                                    "frame camera helper is unavailable".to_owned()
                                })?,
                                bind_group: resolved_bind_groups.into_iter().next().ok_or_else(
                                    || "camera helper requires a bind group".to_owned(),
                                )?,
                            }),
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(LogicalOp::Render(LogicalPass {
                    target,
                    load_op: planned_pass.load_op,
                    clear_color: planned_pass.clear_color,
                    depth: planned_pass.depth,
                    commands,
                }))
            }
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn encode_logical_op(
    encoder: &mut wgpu::CommandEncoder,
    canvas_view: &wgpu::TextureView,
    canvas_target: &RefCell<CanvasTarget>,
    logical_op: LogicalOp,
) {
    match logical_op {
        LogicalOp::Compute(command) => encode_compute_command(encoder, command),
        LogicalOp::Render(logical_pass) => {
            encode_logical_pass(encoder, canvas_view, canvas_target, logical_pass);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn encode_compute_command(encoder: &mut wgpu::CommandEncoder, command: ComputeCommand) {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("BelfastComputePass"),
        timestamp_writes: None,
    });
    if let Some(bind_group) = command.bind_group.as_ref() {
        bind_group
            .bind_group()
            .bind_compute(&mut pass, bind_group.group_index());
    }
    command
        .compute
        .compute()
        .dispatch(&mut pass, command.workgroups);
}

#[cfg(target_arch = "wasm32")]
fn encode_logical_pass(
    encoder: &mut wgpu::CommandEncoder,
    canvas_view: &wgpu::TextureView,
    canvas_target: &RefCell<CanvasTarget>,
    logical_pass: LogicalPass,
) {
    let LogicalPass {
        target,
        load_op,
        clear_color,
        depth,
        commands,
    } = logical_pass;
    let color_load = color_load_op(load_op, clear_color);
    let depth_load = depth_load_op(load_op);
    let draw_meshes: Vec<_> = commands
        .iter()
        .map(|command| match command {
            RenderCommand::Draw { draw, .. } => Some(draw.mesh()),
            RenderCommand::Axis { .. }
            | RenderCommand::Ball { .. }
            | RenderCommand::Camera { .. } => None,
        })
        .collect();

    match target {
        FrameTarget::Canvas => {
            let canvas_target = canvas_target.borrow();
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: canvas_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: color_load,
                    store: wgpu::StoreOp::Store,
                },
            })];
            let depth_stencil_attachment = if depth {
                canvas_target
                    .depth_view()
                    .map(|view| wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: depth_load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    })
            } else {
                None
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("BelfastCanvasRenderPass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            encode_render_commands(&mut pass, &commands, &draw_meshes);
        }
        FrameTarget::RenderTarget(target) => {
            let target = target.borrow();
            let mut pass = target.begin_render_pass(
                encoder,
                belfast::RenderPassOptions {
                    clear_color,
                    load_op: color_load,
                    ..Default::default()
                },
            );
            encode_render_commands(&mut pass, &commands, &draw_meshes);
        }
        FrameTarget::ShadowMap(shadow_map) => {
            let mut pass = shadow_map.texture().begin_depth_pass(encoder, depth_load);
            encode_render_commands(&mut pass, &commands, &draw_meshes);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn encode_render_commands<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    commands: &'pass [RenderCommand],
    draw_meshes: &'pass [Option<Ref<'pass, belfast::Mesh>>],
) {
    for (command, mesh) in commands.iter().zip(draw_meshes) {
        match command {
            RenderCommand::Draw {
                draw,
                bind_groups,
                instance_count,
            } => {
                for bind_group in bind_groups {
                    bind_group.bind_group().bind(pass, bind_group.group_index());
                }
                draw.draw().draw(
                    pass,
                    mesh.as_ref().expect("draw command requires a mesh"),
                    *instance_count,
                );
            }
            RenderCommand::Axis { axis, bind_group } => {
                axis.helper().draw(pass, bind_group.bind_group());
            }
            RenderCommand::Ball { ball, bind_group } => {
                ball.helper().draw(pass, bind_group.bind_group());
            }
            RenderCommand::Camera { camera, bind_group } => {
                camera.helper().draw(pass, bind_group.bind_group());
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn rc_key<T>(value: &Rc<T>) -> u64 {
    Rc::as_ptr(value) as usize as u64
}

#[cfg(target_arch = "wasm32")]
fn device_key(device: &belfast::Device) -> DeviceKey {
    DeviceKey(device.gpu() as *const wgpu::Device as usize as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear(value: f64) -> wgpu::Color {
        wgpu::Color {
            r: value,
            g: value,
            b: value,
            a: 1.0,
        }
    }

    fn draw(key: u64, device: u64, bind_group_count: usize) -> DrawValidation {
        DrawValidation {
            key: DrawKey(key),
            device: DeviceKey(device),
            bind_group_count,
        }
    }

    fn bind_group(key: u64, draw: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: Some(DrawKey(draw)),
            compute: None,
            axis: None,
            camera: None,
            ball: None,
            device: DeviceKey(device),
            group_index: 0,
        }
    }

    fn shadow_bind_group(key: u64, draw: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: Some(DrawKey(draw)),
            compute: None,
            axis: None,
            camera: None,
            ball: None,
            device: DeviceKey(device),
            group_index: 1,
        }
    }

    fn compute_bind_group(key: u64, compute: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: None,
            compute: Some(ComputeKey(compute)),
            axis: None,
            camera: None,
            ball: None,
            device: DeviceKey(device),
            group_index: 0,
        }
    }

    fn axis_bind_group(key: u64, axis: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: None,
            compute: None,
            axis: Some(AxisKey(axis)),
            camera: None,
            ball: None,
            device: DeviceKey(device),
            group_index: 0,
        }
    }

    fn axis(key: u64, device: u64) -> AxisValidation {
        AxisValidation {
            key: AxisKey(key),
            device: DeviceKey(device),
        }
    }

    fn compute(key: u64, device: u64, requires_bind_group: bool) -> ComputeValidation {
        ComputeValidation {
            key: ComputeKey(key),
            device: DeviceKey(device),
            requires_bind_group,
        }
    }

    fn render_pass(op: &PlannedOp) -> &PlannedRenderPass {
        match op {
            PlannedOp::RenderPass(pass) => pass,
            PlannedOp::Compute { .. } => panic!("expected render pass"),
        }
    }

    #[test]
    fn binding_a_new_target_closes_the_previous_logical_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Offscreen(7), clear(0.1), false);
        plan.render(DrawKey(1), vec![], 1).unwrap();
        plan.bind_target(TargetKey::Canvas, clear(0.0), false);
        plan.render(DrawKey(2), vec![BindGroupKey(3)], 1).unwrap();

        let ops = plan.finish().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(render_pass(&ops[0]).target, TargetKey::Offscreen(7));
        assert_eq!(render_pass(&ops[0]).load_op, PlannedLoadOp::Clear);
        assert_eq!(render_pass(&ops[1]).target, TargetKey::Canvas);
        assert_eq!(render_pass(&ops[1]).load_op, PlannedLoadOp::Clear);
    }

    #[test]
    fn render_without_bind_defaults_to_canvas() {
        let mut plan = FramePlan::new();
        plan.render(DrawKey(1), vec![], 1).unwrap();
        assert_eq!(
            render_pass(&plan.finish().unwrap()[0]).target,
            TargetKey::Canvas
        );
    }

    #[test]
    fn empty_frame_is_rejected() {
        assert_eq!(
            FramePlan::new().finish(),
            Err("frame contains no commands".into())
        );
    }

    #[test]
    fn compute_only_frame_is_valid() {
        let mut plan = FramePlan::new();
        plan.dispatch(ComputeKey(1), None, [1, 1, 1]).unwrap();
        assert_eq!(
            plan.finish().unwrap(),
            vec![PlannedOp::Compute {
                compute: ComputeKey(1),
                bind_group: None,
                workgroups: [1, 1, 1],
            }]
        );
    }

    #[test]
    fn compute_splits_render_pass_and_reopens_with_load() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Shadow(9), clear(0.2), true);
        plan.render(DrawKey(1), vec![], 1).unwrap();
        plan.dispatch(ComputeKey(7), Some(BindGroupKey(8)), [4, 1, 1])
            .unwrap();
        plan.render(DrawKey(2), vec![], 1).unwrap();

        let ops = plan.finish().unwrap();
        assert_eq!(ops.len(), 3);
        assert_eq!(render_pass(&ops[0]).load_op, PlannedLoadOp::Clear);
        assert_eq!(
            ops[1],
            PlannedOp::Compute {
                compute: ComputeKey(7),
                bind_group: Some(BindGroupKey(8)),
                workgroups: [4, 1, 1],
            }
        );
        assert!(matches!(
            &ops[2],
            PlannedOp::RenderPass(PlannedRenderPass {
                target: TargetKey::Shadow(_),
                load_op: PlannedLoadOp::Load,
                ..
            })
        ));
    }

    #[test]
    fn bind_target_after_compute_clears_again() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2), false);
        plan.render(DrawKey(1), vec![], 1).unwrap();
        plan.dispatch(ComputeKey(7), None, [1, 1, 1]).unwrap();
        plan.bind_target(TargetKey::Canvas, clear(0.9), false);
        plan.render(DrawKey(2), vec![], 1).unwrap();

        let ops = plan.finish().unwrap();
        assert_eq!(render_pass(&ops[2]).load_op, PlannedLoadOp::Clear);
        assert_eq!(render_pass(&ops[2]).clear_color, clear(0.9));
    }

    #[test]
    fn missing_required_bind_group_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, 1), &[]),
            Err("draw requires a bind group".into())
        );
        assert_eq!(
            validate_dispatch_command(DeviceKey(1), compute(2, 1, true), None),
            Err("compute requires a bind group".into())
        );
    }

    #[test]
    fn resource_free_draw_accepts_zero_bind_groups() {
        assert!(validate_render_command(DeviceKey(1), draw(7, 1, 0), &[]).is_ok());
    }

    #[test]
    fn particle_image_draw_requires_each_of_three_groups() {
        let camera = bind_group(10, 7, 1);
        let shadow = shadow_bind_group(11, 7, 1);
        let image = BindGroupValidation {
            group_index: 2,
            ..bind_group(12, 7, 1)
        };
        assert!(
            validate_render_command(DeviceKey(1), draw(7, 1, 3), &[image, camera, shadow]).is_ok()
        );
        for groups in [
            vec![camera, shadow],
            vec![camera, shadow, shadow],
            vec![image, image, image],
        ] {
            assert!(validate_render_command(DeviceKey(1), draw(7, 1, 3), &groups).is_err());
        }
        let wrong_draw = BindGroupValidation {
            draw: Some(DrawKey(99)),
            ..image
        };
        let wrong_device = BindGroupValidation {
            device: DeviceKey(99),
            ..image
        };
        for invalid in [wrong_draw, wrong_device] {
            assert!(validate_render_command(
                DeviceKey(1),
                draw(7, 1, 3),
                &[camera, shadow, invalid]
            )
            .is_err());
        }
    }

    #[test]
    fn particle_transition_requires_four_distinct_groups() {
        let camera = bind_group(10, 7, 1);
        let shadow = shadow_bind_group(11, 7, 1);
        let a = BindGroupValidation {
            group_index: 2,
            ..bind_group(12, 7, 1)
        };
        let b = BindGroupValidation {
            group_index: 3,
            ..bind_group(13, 7, 1)
        };
        assert!(
            validate_render_command(DeviceKey(1), draw(7, 1, 4), &[b, a, camera, shadow]).is_ok()
        );
        for groups in [
            vec![camera, shadow, a],
            vec![camera, shadow, a, a],
            vec![b, b, b, b],
        ] {
            assert!(validate_render_command(DeviceKey(1), draw(7, 1, 4), &groups).is_err());
        }
    }

    #[test]
    fn unexpected_bind_group_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, 0), &[bind_group(3, 2, 1)]),
            Err("draw does not accept a bind group".into())
        );
        assert_eq!(
            validate_dispatch_command(
                DeviceKey(1),
                compute(2, 1, false),
                Some(compute_bind_group(3, 2, 1)),
            ),
            Err("compute does not accept a bind group".into())
        );
    }

    #[test]
    fn bind_group_for_a_different_draw_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, 1), &[bind_group(3, 4, 1)]),
            Err("bind group was created for a different draw".into())
        );
        assert_eq!(
            validate_dispatch_command(
                DeviceKey(1),
                compute(2, 1, true),
                Some(compute_bind_group(3, 4, 1)),
            ),
            Err("bind group was created for a different compute".into())
        );
    }

    #[test]
    fn instance_counts_survive_pass_splits_and_depth_is_preserved() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2), true);
        plan.render(DrawKey(1), vec![], 64).unwrap();
        plan.dispatch(ComputeKey(7), None, [1, 1, 1]).unwrap();
        plan.render(DrawKey(2), vec![], 128).unwrap();

        let ops = plan.finish().unwrap();
        assert_eq!(ops.len(), 3);
        let first = render_pass(&ops[0]);
        assert!(first.depth);
        assert_eq!(first.load_op, PlannedLoadOp::Clear);
        assert_eq!(first.commands[0].instance_count, 64);
        let second = render_pass(&ops[2]);
        assert!(second.depth);
        assert_eq!(second.load_op, PlannedLoadOp::Load);
        assert_eq!(second.commands[0].instance_count, 128);
    }

    #[test]
    fn bind_target_depth_is_used_on_the_next_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.1), true);
        plan.render(DrawKey(1), vec![], 1).unwrap();

        let ops = plan.finish().unwrap();
        let pass = render_pass(&ops[0]);
        assert!(pass.depth);
        assert_eq!(pass.commands[0].instance_count, 1);
    }

    #[test]
    fn axis_helper_joins_the_current_render_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2), false);
        plan.render(DrawKey(1), vec![], 1).unwrap();
        plan.render_axis(AxisKey(2), BindGroupKey(3)).unwrap();

        let ops = plan.finish().unwrap();
        let commands = &render_pass(&ops[0]).commands;
        assert_eq!(commands.len(), 2);
        assert_eq!(
            commands[1],
            PlannedRenderCommand {
                drawable: PlannedDrawable::Axis(AxisKey(2)),
                bind_groups: vec![BindGroupKey(3)],
                instance_count: 1,
            }
        );
    }

    #[test]
    fn ball_helper_joins_the_current_render_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2), false);
        plan.render(DrawKey(1), vec![], 1).unwrap();
        plan.render_ball(BallKey(2), BindGroupKey(3)).unwrap();

        let ops = plan.finish().unwrap();
        let commands = &render_pass(&ops[0]).commands;
        assert_eq!(commands.len(), 2);
        assert_eq!(
            commands[1],
            PlannedRenderCommand {
                drawable: PlannedDrawable::Ball(BallKey(2)),
                bind_groups: vec![BindGroupKey(3)],
                instance_count: 1,
            }
        );
    }

    #[test]
    fn camera_helper_joins_the_current_render_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2), false);
        plan.render(DrawKey(1), vec![], 1).unwrap();
        plan.render_camera(CameraKey(2), BindGroupKey(3)).unwrap();

        let ops = plan.finish().unwrap();
        let commands = &render_pass(&ops[0]).commands;
        assert_eq!(commands.len(), 2);
        assert_eq!(
            commands[1],
            PlannedRenderCommand {
                drawable: PlannedDrawable::Camera(CameraKey(2)),
                bind_groups: vec![BindGroupKey(3)],
                instance_count: 1,
            }
        );
    }

    fn camera_bind_group(key: u64, camera: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: None,
            compute: None,
            axis: None,
            ball: None,
            camera: Some(CameraKey(camera)),
            device: DeviceKey(device),
            group_index: 0,
        }
    }

    fn camera(key: u64, device: u64) -> CameraValidation {
        CameraValidation {
            key: CameraKey(key),
            device: DeviceKey(device),
        }
    }

    #[test]
    fn camera_helper_requires_a_matching_bind_group() {
        assert_eq!(
            validate_camera_render_command(DeviceKey(1), camera(2, 1), None),
            Err("camera helper requires a bind group".into())
        );
        assert_eq!(
            validate_camera_render_command(
                DeviceKey(1),
                camera(2, 1),
                Some(camera_bind_group(3, 4, 1)),
            ),
            Err("bind group was created for a different camera helper".into())
        );
    }

    #[test]
    fn axis_helper_requires_its_own_bind_group() {
        assert_eq!(
            validate_axis_render_command(DeviceKey(1), axis(2, 1), None),
            Err("axis helper requires a bind group".into())
        );
        assert_eq!(
            validate_axis_render_command(DeviceKey(1), axis(2, 1), Some(axis_bind_group(3, 4, 1))),
            Err("bind group was created for a different axis helper".into())
        );
        assert_eq!(
            validate_axis_render_command(DeviceKey(1), axis(2, 9), Some(axis_bind_group(3, 2, 1))),
            Err("axis helper was created by a different device".into())
        );
    }

    fn ball_bind_group(key: u64, ball: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: None,
            compute: None,
            axis: None,
            ball: Some(BallKey(ball)),
            camera: None,
            device: DeviceKey(device),
            group_index: 0,
        }
    }

    fn ball(key: u64, device: u64) -> BallValidation {
        BallValidation {
            key: BallKey(key),
            device: DeviceKey(device),
        }
    }

    #[test]
    fn ball_helper_requires_its_own_bind_group() {
        assert_eq!(
            validate_ball_render_command(DeviceKey(1), ball(2, 1), None),
            Err("ball helper requires a bind group".into())
        );
        assert_eq!(
            validate_ball_render_command(DeviceKey(1), ball(2, 1), Some(ball_bind_group(3, 4, 1))),
            Err("bind group was created for a different ball helper".into())
        );
        assert_eq!(
            validate_ball_render_command(DeviceKey(1), ball(2, 9), Some(ball_bind_group(3, 2, 1))),
            Err("ball helper was created by a different device".into())
        );
    }

    #[test]
    fn two_bind_groups_must_cover_group0_and_group1() {
        assert!(validate_render_command(
            DeviceKey(1),
            draw(2, 1, 2),
            &[bind_group(3, 2, 1), shadow_bind_group(4, 2, 1)],
        )
        .is_ok());
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, 2), &[bind_group(3, 2, 1)]),
            Err("draw requires 2 bind groups".into())
        );
        assert_eq!(
            validate_render_command(
                DeviceKey(1),
                draw(2, 1, 2),
                &[bind_group(3, 2, 1), bind_group(4, 2, 1)],
            ),
            Err("draw requires bind groups for @group(0) and @group(1)".into())
        );
    }

    #[test]
    fn shadow_target_splits_from_the_canvas_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Shadow(9), clear(0.0), true);
        plan.render(DrawKey(1), vec![BindGroupKey(2)], 64).unwrap();
        plan.bind_target(TargetKey::Canvas, clear(0.08), true);
        plan.render(DrawKey(3), vec![BindGroupKey(4), BindGroupKey(5)], 64)
            .unwrap();

        let ops = plan.finish().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(render_pass(&ops[0]).target, TargetKey::Shadow(9));
        assert!(render_pass(&ops[0]).depth);
        assert_eq!(render_pass(&ops[0]).commands[0].instance_count, 64);
        assert_eq!(render_pass(&ops[1]).target, TargetKey::Canvas);
        assert_eq!(
            render_pass(&ops[1]).commands[0].bind_groups,
            vec![BindGroupKey(4), BindGroupKey(5)]
        );
    }

    #[test]
    fn resource_from_a_different_device_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 9, 0), &[]),
            Err("draw was created by a different device".into())
        );
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, 1), &[bind_group(3, 2, 9)]),
            Err("bind group was created by a different device".into())
        );
        assert_eq!(
            validate_dispatch_command(DeviceKey(1), compute(2, 9, false), None),
            Err("compute was created by a different device".into())
        );
        assert_eq!(
            validate_dispatch_command(
                DeviceKey(1),
                compute(2, 1, true),
                Some(compute_bind_group(3, 2, 9)),
            ),
            Err("bind group was created by a different device".into())
        );
        assert_eq!(
            validate_render_target(
                DeviceKey(1),
                wgpu::TextureFormat::Bgra8UnormSrgb,
                RenderTargetValidation {
                    device: DeviceKey(9),
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                },
            ),
            Err("render target was created by a different device".into())
        );
    }

    #[test]
    fn mismatched_render_target_format_is_rejected() {
        assert_eq!(
            validate_render_target(
                DeviceKey(1),
                wgpu::TextureFormat::Bgra8UnormSrgb,
                RenderTargetValidation {
                    device: DeviceKey(1),
                    format: wgpu::TextureFormat::Rgba8Unorm,
                },
            ),
            Err("render target format does not match frame format".into())
        );
    }

    #[test]
    fn non_finite_clear_color_components_are_rejected() {
        for color in [
            wgpu::Color {
                r: f64::NAN,
                ..clear(0.0)
            },
            wgpu::Color {
                g: f64::INFINITY,
                ..clear(0.0)
            },
            wgpu::Color {
                b: f64::NEG_INFINITY,
                ..clear(0.0)
            },
            wgpu::Color {
                a: f64::NAN,
                ..clear(0.0)
            },
        ] {
            assert_eq!(
                validate_clear_color(color),
                Err("clearColor components must be finite".into())
            );
        }
    }

    #[test]
    fn optional_wrapper_pointer_must_be_a_live_wasm_pointer() {
        assert_eq!(validate_live_wrapper_pointer(Some(1.0)), Ok(1));
        assert_eq!(
            validate_live_wrapper_pointer(Some(f64::from(u32::MAX))),
            Ok(u32::MAX)
        );

        for pointer in [
            None,
            Some(0.0),
            Some(-1.0),
            Some(1.5),
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(f64::from(u32::MAX) + 1.0),
        ] {
            assert_eq!(
                validate_live_wrapper_pointer(pointer),
                Err("wrapper must contain a live wasm pointer".into())
            );
        }
    }

    #[test]
    fn optional_wrapper_requires_registered_identity_and_original_pointer() {
        assert_eq!(
            validate_registered_wrapper_pointer(Some(7.0), Some(7.0)),
            Ok(7)
        );

        for (current, registered) in [
            (Some(7.0), None),
            (Some(8.0), Some(7.0)),
            (Some(0.0), Some(7.0)),
        ] {
            assert_eq!(
                validate_registered_wrapper_pointer(current, registered),
                Err("wrapper must be registered with its original live wasm pointer".into())
            );
        }
    }
}
