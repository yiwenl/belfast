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
    bind_group::BindGroupState,
    compute::{parse_workgroups, ComputeState},
    device::{CanvasTarget, PendingGpuErrors, SurfaceLeaseGuard},
    draw::{DrawState, ShaderResourceLayout},
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DeviceKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct DrawKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct AxisKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ComputeKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct BindGroupKey(u64);

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
enum TargetKey {
    #[default]
    Canvas,
    Offscreen(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DrawValidation {
    key: DrawKey,
    device: DeviceKey,
    requires_bind_group: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AxisValidation {
    key: AxisKey,
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
    device: DeviceKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RenderTargetValidation {
    device: DeviceKey,
    format: wgpu::TextureFormat,
}

fn validate_render_command(
    frame_device: DeviceKey,
    draw: DrawValidation,
    bind_group: Option<BindGroupValidation>,
) -> Result<(), String> {
    if draw.device != frame_device {
        return Err("draw was created by a different device".into());
    }

    match (draw.requires_bind_group, bind_group) {
        (true, None) => Err("draw requires a bind group".into()),
        (false, Some(_)) => Err("draw does not accept a bind group".into()),
        (false, None) => Ok(()),
        (true, Some(bind_group)) => {
            if bind_group.device != frame_device {
                return Err("bind group was created by a different device".into());
            }
            if bind_group.draw != Some(draw.key) {
                return Err("bind group was created for a different draw".into());
            }
            Ok(())
        }
    }
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
}

#[derive(Debug, PartialEq)]
struct PlannedRenderCommand {
    drawable: PlannedDrawable,
    bind_group: Option<BindGroupKey>,
}

#[derive(Debug, PartialEq)]
struct PlannedRenderPass {
    target: TargetKey,
    load_op: PlannedLoadOp,
    clear_color: wgpu::Color,
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
    },
    Render {
        drawable: PlannedDrawable,
        bind_group: Option<BindGroupKey>,
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

    fn bind_target(&mut self, target: TargetKey, clear_color: wgpu::Color) {
        self.commands.push(RecordedCommand::BindTarget {
            target,
            clear_color,
        });
    }

    fn render(&mut self, draw: DrawKey, bind_group: Option<BindGroupKey>) -> Result<(), String> {
        self.commands.push(RecordedCommand::Render {
            drawable: PlannedDrawable::Draw(draw),
            bind_group,
        });
        Ok(())
    }

    fn render_axis(&mut self, axis: AxisKey, bind_group: BindGroupKey) -> Result<(), String> {
        self.commands.push(RecordedCommand::Render {
            drawable: PlannedDrawable::Axis(axis),
            bind_group: Some(bind_group),
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
        let mut bound_target: Option<(TargetKey, wgpu::Color)> = None;
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
                } => {
                    flush_pass(&mut current_pass, &mut ops, &mut next_load);
                    bound_target = Some((target, clear_color));
                    next_load.insert(target, PlannedLoadOp::Clear);
                }
                RecordedCommand::Render {
                    drawable,
                    bind_group,
                } => {
                    let (target, clear_color) =
                        bound_target.unwrap_or_else(|| (TargetKey::Canvas, default_clear_color()));
                    if bound_target.is_none() {
                        bound_target = Some((target, clear_color));
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
                            commands: Vec::new(),
                        });
                    }
                    current_pass
                        .as_mut()
                        .expect("frame plan has a current pass")
                        .commands
                        .push(PlannedRenderCommand {
                            drawable,
                            bind_group,
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
fn parse_clear_color(options: JsValue) -> Result<wgpu::Color, JsValue> {
    let options = if options.is_undefined() {
        FramePassOptionsInput::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(to_js_error)?
    };
    validate_clear_color(
        options
            .clear_color
            .map(Into::into)
            .unwrap_or_else(default_clear_color),
    )
    .map_err(to_js_error)
}

#[cfg(target_arch = "wasm32")]
fn optional_render_target(value: JsValue) -> Result<Option<WasmRenderTarget>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    let handle = borrow_registered_wasm_class(
        &value,
        &RENDER_TARGET_WRAPPERS,
        "target must be a live RenderTarget or null",
    )?;
    WasmRenderTarget::try_from_js_value(handle)
        .map(Some)
        .map_err(|_| to_js_error("target must be a live RenderTarget or null"))
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
pub(crate) enum DrawOrAxisHelper {
    Draw(WasmDraw),
    Axis(WasmAxisHelper),
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn parse_draw_or_axis_helper(value: &JsValue) -> Result<DrawOrAxisHelper, JsValue> {
    const ERROR: &str = "value must be a live Draw or AxisHelper";
    let draw_handle = clone_live_wasm_class(value, ERROR)?;
    if let Ok(draw) = WasmDraw::try_from_js_value(draw_handle) {
        return Ok(DrawOrAxisHelper::Draw(draw));
    }
    let axis_handle = clone_live_wasm_class(value, ERROR)?;
    WasmAxisHelper::try_from_js_value(axis_handle)
        .map(DrawOrAxisHelper::Axis)
        .map_err(|_| to_js_error(ERROR))
}

#[cfg(target_arch = "wasm32")]
enum FrameTarget {
    Canvas,
    RenderTarget(Rc<RefCell<belfast::RenderTarget>>),
}

#[cfg(target_arch = "wasm32")]
enum RenderCommand {
    Draw {
        draw: Rc<DrawState>,
        bind_group: Option<Rc<BindGroupState>>,
    },
    Axis {
        axis: Rc<AxisHelperState>,
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
    commands: Vec<RenderCommand>,
}

#[cfg(target_arch = "wasm32")]
enum LogicalOp {
    Compute(ComputeCommand),
    Render(LogicalPass),
}

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
    draws: HashMap<DrawKey, Rc<DrawState>>,
    axes: HashMap<AxisKey, Rc<AxisHelperState>>,
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
            draws: HashMap::new(),
            axes: HashMap::new(),
            computes: HashMap::new(),
            bind_groups: HashMap::new(),
        }
    }

    pub(crate) fn render_draw(
        &mut self,
        draw: &WasmDraw,
        bind_group: JsValue,
    ) -> Result<(), JsValue> {
        let bind_group_handle = optional_bind_group(bind_group)?;
        let bind_group = bind_group_handle.as_ref();
        let draw_key = DrawKey(rc_key(&draw.state));
        let bind_group_validation = bind_group.map(bind_group_validation);
        validate_render_command(
            device_key(&self.device),
            DrawValidation {
                key: draw_key,
                device: device_key(draw.state.draw().device()),
                requires_bind_group: !matches!(&draw.state.resources, ShaderResourceLayout::None),
            },
            bind_group_validation,
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
        if let Some(bind_group) = bind_group {
            self.bind_groups.insert(
                BindGroupKey(rc_key(&bind_group.state)),
                bind_group.state.clone(),
            );
        }
        self.plan
            .render(draw_key, bind_group_validation.map(|binding| binding.key))
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
        #[wasm_bindgen(unchecked_optional_param_type = "RenderTarget | null")] target: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "unknown")] options: JsValue,
    ) -> Result<(), JsValue> {
        let clear_color = parse_clear_color(options)?;
        let target_handle = optional_render_target(target)?;
        let target = if let Some(target) = target_handle.as_ref() {
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
        } else {
            TargetKey::Canvas
        };
        self.plan.bind_target(target, clear_color);
        Ok(())
    }

    pub fn render(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Draw | AxisHelper")] drawable: JsValue,
        #[wasm_bindgen(
            js_name = bindGroup,
            unchecked_optional_param_type = "BindGroup | null"
        )]
        bind_group: JsValue,
    ) -> Result<(), JsValue> {
        match parse_draw_or_axis_helper(&drawable)? {
            DrawOrAxisHelper::Draw(draw) => self.render_draw(&draw, bind_group),
            DrawOrAxisHelper::Axis(axes) => self.render_axis(&axes, bind_group),
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
            &self.draws,
            &self.axes,
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

        for logical_op in logical_ops {
            encode_logical_op(&mut encoder, &self.canvas_view, logical_op);
        }

        self.device.queue().submit([encoder.finish()]);
        let surface_texture = self
            .surface_texture
            .take()
            .ok_or_else(|| to_js_error("frame surface texture is unavailable"))?;
        surface_texture.present();
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
        device: device_key(bind_group.state.bind_group().device()),
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
    draws: &HashMap<DrawKey, Rc<DrawState>>,
    axes: &HashMap<AxisKey, Rc<AxisHelperState>>,
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
                };
                let commands = planned_pass
                    .commands
                    .into_iter()
                    .map(|command| {
                        let bind_group = command
                            .bind_group
                            .map(|key| {
                                bind_groups
                                    .get(&key)
                                    .cloned()
                                    .ok_or_else(|| "frame bind group is unavailable".to_owned())
                            })
                            .transpose()?;
                        match command.drawable {
                            PlannedDrawable::Draw(draw) => Ok(RenderCommand::Draw {
                                draw: draws
                                    .get(&draw)
                                    .cloned()
                                    .ok_or_else(|| "frame draw is unavailable".to_owned())?,
                                bind_group,
                            }),
                            PlannedDrawable::Axis(axis) => Ok(RenderCommand::Axis {
                                axis: axes
                                    .get(&axis)
                                    .cloned()
                                    .ok_or_else(|| "frame axis helper is unavailable".to_owned())?,
                                bind_group: bind_group.ok_or_else(|| {
                                    "axis helper requires a bind group".to_owned()
                                })?,
                            }),
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(LogicalOp::Render(LogicalPass {
                    target,
                    load_op: planned_pass.load_op,
                    clear_color: planned_pass.clear_color,
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
    logical_op: LogicalOp,
) {
    match logical_op {
        LogicalOp::Compute(command) => encode_compute_command(encoder, command),
        LogicalOp::Render(logical_pass) => {
            encode_logical_pass(encoder, canvas_view, logical_pass);
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
    logical_pass: LogicalPass,
) {
    let LogicalPass {
        target,
        load_op,
        clear_color,
        commands,
    } = logical_pass;
    let color_load = color_load_op(load_op, clear_color);
    let draw_meshes: Vec<_> = commands
        .iter()
        .map(|command| match command {
            RenderCommand::Draw { draw, .. } => Some(draw.mesh()),
            RenderCommand::Axis { .. } => None,
        })
        .collect();

    match target {
        FrameTarget::Canvas => {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: canvas_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: color_load,
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("BelfastCanvasRenderPass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
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
                    depth_load_op: match load_op {
                        PlannedLoadOp::Clear => wgpu::LoadOp::Clear(1.0),
                        PlannedLoadOp::Load => wgpu::LoadOp::Load,
                    },
                    ..Default::default()
                },
            );
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
            RenderCommand::Draw { draw, bind_group } => {
                if let Some(bind_group) = bind_group.as_ref() {
                    bind_group.bind_group().bind(pass, bind_group.group_index());
                }
                draw.draw().draw(
                    pass,
                    mesh.as_ref().expect("draw command requires a mesh"),
                    1,
                );
            }
            RenderCommand::Axis { axis, bind_group } => {
                axis.helper().draw(pass, bind_group.bind_group());
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

    fn draw(key: u64, device: u64, requires_bind_group: bool) -> DrawValidation {
        DrawValidation {
            key: DrawKey(key),
            device: DeviceKey(device),
            requires_bind_group,
        }
    }

    fn bind_group(key: u64, draw: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: Some(DrawKey(draw)),
            compute: None,
            axis: None,
            device: DeviceKey(device),
        }
    }

    fn compute_bind_group(key: u64, compute: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: None,
            compute: Some(ComputeKey(compute)),
            axis: None,
            device: DeviceKey(device),
        }
    }

    fn axis_bind_group(key: u64, axis: u64, device: u64) -> BindGroupValidation {
        BindGroupValidation {
            key: BindGroupKey(key),
            draw: None,
            compute: None,
            axis: Some(AxisKey(axis)),
            device: DeviceKey(device),
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
        plan.bind_target(TargetKey::Offscreen(7), clear(0.1));
        plan.render(DrawKey(1), None).unwrap();
        plan.bind_target(TargetKey::Canvas, clear(0.0));
        plan.render(DrawKey(2), Some(BindGroupKey(3))).unwrap();

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
        plan.render(DrawKey(1), None).unwrap();
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
        plan.bind_target(TargetKey::Canvas, clear(0.2));
        plan.render(DrawKey(1), None).unwrap();
        plan.dispatch(ComputeKey(7), Some(BindGroupKey(8)), [4, 1, 1])
            .unwrap();
        plan.render(DrawKey(2), None).unwrap();

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
        assert_eq!(render_pass(&ops[2]).load_op, PlannedLoadOp::Load);
        assert_eq!(render_pass(&ops[2]).target, TargetKey::Canvas);
    }

    #[test]
    fn bind_target_after_compute_clears_again() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2));
        plan.render(DrawKey(1), None).unwrap();
        plan.dispatch(ComputeKey(7), None, [1, 1, 1]).unwrap();
        plan.bind_target(TargetKey::Canvas, clear(0.9));
        plan.render(DrawKey(2), None).unwrap();

        let ops = plan.finish().unwrap();
        assert_eq!(render_pass(&ops[2]).load_op, PlannedLoadOp::Clear);
        assert_eq!(render_pass(&ops[2]).clear_color, clear(0.9));
    }

    #[test]
    fn missing_required_bind_group_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, true), None),
            Err("draw requires a bind group".into())
        );
        assert_eq!(
            validate_dispatch_command(DeviceKey(1), compute(2, 1, true), None),
            Err("compute requires a bind group".into())
        );
    }

    #[test]
    fn unexpected_bind_group_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, false), Some(bind_group(3, 2, 1)),),
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
            validate_render_command(DeviceKey(1), draw(2, 1, true), Some(bind_group(3, 4, 1)),),
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
    fn axis_helper_joins_the_current_render_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Canvas, clear(0.2));
        plan.render(DrawKey(1), None).unwrap();
        plan.render_axis(AxisKey(2), BindGroupKey(3)).unwrap();

        let ops = plan.finish().unwrap();
        let commands = &render_pass(&ops[0]).commands;
        assert_eq!(commands.len(), 2);
        assert_eq!(
            commands[1],
            PlannedRenderCommand {
                drawable: PlannedDrawable::Axis(AxisKey(2)),
                bind_group: Some(BindGroupKey(3)),
            }
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

    #[test]
    fn resource_from_a_different_device_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 9, false), None),
            Err("draw was created by a different device".into())
        );
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, true), Some(bind_group(3, 2, 9)),),
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
