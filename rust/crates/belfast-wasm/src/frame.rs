#[cfg(target_arch = "wasm32")]
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{convert::TryFromJsValue, prelude::*, JsCast};

#[cfg(target_arch = "wasm32")]
use crate::{
    bind_group::BindGroupState,
    device::{CanvasTarget, PendingGpuErrors},
    draw::{DrawState, ShaderResourceLayout},
    to_js_error, WasmBindGroup, WasmDraw, WasmRenderTarget,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DeviceKey(u64);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct DrawKey(u64);

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
struct BindGroupValidation {
    key: BindGroupKey,
    draw: DrawKey,
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
            let BindGroupValidation {
                key: _,
                draw: bind_group_draw,
                device: bind_group_device,
            } = bind_group;
            if bind_group_device != frame_device {
                return Err("bind group was created by a different device".into());
            }
            if bind_group_draw != draw.key {
                return Err("bind group was created for a different draw".into());
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

#[derive(Debug, PartialEq)]
struct PlannedRenderCommand {
    draw: DrawKey,
    bind_group: Option<BindGroupKey>,
}

#[derive(Debug, PartialEq)]
struct PlannedLogicalPass {
    target: TargetKey,
    clear_color: wgpu::Color,
    commands: Vec<PlannedRenderCommand>,
}

struct FramePlan {
    passes: Vec<PlannedLogicalPass>,
    current: Option<PlannedLogicalPass>,
}

impl FramePlan {
    fn new() -> Self {
        Self {
            passes: Vec::new(),
            current: None,
        }
    }

    fn bind_target(&mut self, target: TargetKey, clear_color: wgpu::Color) {
        if let Some(current) = self.current.take() {
            if !current.commands.is_empty() {
                self.passes.push(current);
            }
        }
        self.current = Some(PlannedLogicalPass {
            target,
            clear_color,
            commands: Vec::new(),
        });
    }

    fn render(&mut self, draw: DrawKey, bind_group: Option<BindGroupKey>) -> Result<(), String> {
        if self.current.is_none() {
            self.bind_target(TargetKey::Canvas, default_clear_color());
        }
        self.current
            .as_mut()
            .expect("frame plan has a current pass")
            .commands
            .push(PlannedRenderCommand { draw, bind_group });
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<PlannedLogicalPass>, String> {
        if let Some(current) = self.current.take() {
            if !current.commands.is_empty() {
                self.passes.push(current);
            }
        }
        if self.passes.is_empty() {
            return Err("frame contains no draw commands".into());
        }
        Ok(self.passes)
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
    let handle = clone_frame_handle(&value, "RenderTarget")?;
    WasmRenderTarget::try_from_js_value(handle)
        .map(Some)
        .map_err(|_| to_js_error("target must be a RenderTarget or null"))
}

#[cfg(target_arch = "wasm32")]
fn optional_bind_group(value: JsValue) -> Result<Option<WasmBindGroup>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    let handle = clone_frame_handle(&value, "BindGroup")?;
    WasmBindGroup::try_from_js_value(handle)
        .map(Some)
        .map_err(|_| to_js_error("bindGroup must be a BindGroup or null"))
}

#[cfg(target_arch = "wasm32")]
fn clone_frame_handle(value: &JsValue, class_name: &str) -> Result<JsValue, JsValue> {
    // wasm-bindgen cannot export Option<&Class>; consume a cloned wrapper, not the caller's.
    let method = js_sys::Reflect::get(value, &JsValue::from_str("__frameHandle"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| to_js_error(format!("value must be a {class_name}")))?;
    method.call0(value)
}

#[cfg(target_arch = "wasm32")]
enum FrameTarget {
    Canvas,
    RenderTarget(Rc<RefCell<belfast::RenderTarget>>),
}

#[cfg(target_arch = "wasm32")]
struct RenderCommand {
    draw: Rc<DrawState>,
    bind_group: Option<Rc<BindGroupState>>,
}

#[cfg(target_arch = "wasm32")]
struct LogicalPass {
    target: FrameTarget,
    clear_color: wgpu::Color,
    commands: Vec<RenderCommand>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = Frame)]
pub struct WasmFrame {
    device: belfast::Device,
    pending_gpu_errors: PendingGpuErrors,
    canvas_target: Rc<RefCell<CanvasTarget>>,
    surface_texture: wgpu::SurfaceTexture,
    canvas_view: wgpu::TextureView,
    reconfigure_after_present: bool,
    plan: FramePlan,
    render_targets: HashMap<TargetKey, Rc<RefCell<belfast::RenderTarget>>>,
    draws: HashMap<DrawKey, Rc<DrawState>>,
    bind_groups: HashMap<BindGroupKey, Rc<BindGroupState>>,
}

#[cfg(target_arch = "wasm32")]
impl WasmFrame {
    pub(crate) fn new(
        device: belfast::Device,
        pending_gpu_errors: PendingGpuErrors,
        canvas_target: Rc<RefCell<CanvasTarget>>,
        surface_texture: wgpu::SurfaceTexture,
        reconfigure_after_present: bool,
    ) -> Self {
        let canvas_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            device,
            pending_gpu_errors,
            canvas_target,
            surface_texture,
            canvas_view,
            reconfigure_after_present,
            plan: FramePlan::new(),
            render_targets: HashMap::new(),
            draws: HashMap::new(),
            bind_groups: HashMap::new(),
        }
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
        draw: &WasmDraw,
        #[wasm_bindgen(
            js_name = bindGroup,
            unchecked_optional_param_type = "BindGroup | null"
        )]
        bind_group: JsValue,
    ) -> Result<(), JsValue> {
        let bind_group_handle = optional_bind_group(bind_group)?;
        let bind_group = bind_group_handle.as_ref();
        let draw_key = DrawKey(rc_key(&draw.state));
        let bind_group_validation = bind_group.map(|bind_group| BindGroupValidation {
            key: BindGroupKey(rc_key(&bind_group.state)),
            draw: DrawKey(rc_key(bind_group.state.draw())),
            device: device_key(bind_group.state.bind_group().device()),
        });
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

    pub fn submit(self) -> Result<(), JsValue> {
        let Self {
            device,
            pending_gpu_errors,
            canvas_target,
            surface_texture,
            canvas_view,
            reconfigure_after_present,
            plan,
            render_targets,
            draws,
            bind_groups,
        } = self;
        let planned_passes = plan.finish().map_err(to_js_error)?;
        let logical_passes =
            materialize_passes(planned_passes, &render_targets, &draws, &bind_groups)
                .map_err(to_js_error)?;
        let mut encoder = device
            .gpu()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("BelfastFrameEncoder"),
            });

        for logical_pass in logical_passes {
            encode_logical_pass(&mut encoder, &canvas_view, logical_pass);
        }

        device.queue().submit([encoder.finish()]);
        surface_texture.present();
        if reconfigure_after_present {
            canvas_target.borrow().configure(device.gpu());
        }
        if let Some(error) = pending_gpu_errors.take() {
            return Err(to_js_error(error));
        }
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
fn materialize_passes(
    planned_passes: Vec<PlannedLogicalPass>,
    render_targets: &HashMap<TargetKey, Rc<RefCell<belfast::RenderTarget>>>,
    draws: &HashMap<DrawKey, Rc<DrawState>>,
    bind_groups: &HashMap<BindGroupKey, Rc<BindGroupState>>,
) -> Result<Vec<LogicalPass>, String> {
    planned_passes
        .into_iter()
        .map(|planned_pass| {
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
                    Ok(RenderCommand {
                        draw: draws
                            .get(&command.draw)
                            .cloned()
                            .ok_or_else(|| "frame draw is unavailable".to_owned())?,
                        bind_group: command
                            .bind_group
                            .map(|key| {
                                bind_groups
                                    .get(&key)
                                    .cloned()
                                    .ok_or_else(|| "frame bind group is unavailable".to_owned())
                            })
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(LogicalPass {
                target,
                clear_color: planned_pass.clear_color,
                commands,
            })
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn encode_logical_pass(
    encoder: &mut wgpu::CommandEncoder,
    canvas_view: &wgpu::TextureView,
    logical_pass: LogicalPass,
) {
    let LogicalPass {
        target,
        clear_color,
        commands,
    } = logical_pass;
    let meshes: Vec<_> = commands.iter().map(|command| command.draw.mesh()).collect();

    match target {
        FrameTarget::Canvas => {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: canvas_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
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
            encode_render_commands(&mut pass, &commands, &meshes);
        }
        FrameTarget::RenderTarget(target) => {
            let target = target.borrow();
            let mut pass = target.begin_render_pass(
                encoder,
                belfast::RenderPassOptions {
                    clear_color,
                    load_op: wgpu::LoadOp::Clear(clear_color),
                    ..Default::default()
                },
            );
            encode_render_commands(&mut pass, &commands, &meshes);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn encode_render_commands<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    commands: &'pass [RenderCommand],
    meshes: &'pass [Ref<'pass, belfast::Mesh>],
) {
    for (command, mesh) in commands.iter().zip(meshes) {
        if let Some(bind_group) = command.bind_group.as_ref() {
            bind_group.bind_group().bind(pass, bind_group.group_index());
        }
        command.draw.draw().draw(pass, mesh, 1);
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
            draw: DrawKey(draw),
            device: DeviceKey(device),
        }
    }

    #[test]
    fn binding_a_new_target_closes_the_previous_logical_pass() {
        let mut plan = FramePlan::new();
        plan.bind_target(TargetKey::Offscreen(7), clear(0.1));
        plan.render(DrawKey(1), None).unwrap();
        plan.bind_target(TargetKey::Canvas, clear(0.0));
        plan.render(DrawKey(2), Some(BindGroupKey(3))).unwrap();

        let passes = plan.finish().unwrap();
        assert_eq!(passes.len(), 2);
        assert_eq!(passes[0].target, TargetKey::Offscreen(7));
        assert_eq!(passes[1].target, TargetKey::Canvas);
    }

    #[test]
    fn render_without_bind_defaults_to_canvas() {
        let mut plan = FramePlan::new();
        plan.render(DrawKey(1), None).unwrap();
        assert_eq!(plan.finish().unwrap()[0].target, TargetKey::Canvas);
    }

    #[test]
    fn empty_frame_is_rejected() {
        assert_eq!(
            FramePlan::new().finish(),
            Err("frame contains no draw commands".into())
        );
    }

    #[test]
    fn missing_required_bind_group_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, true), None),
            Err("draw requires a bind group".into())
        );
    }

    #[test]
    fn unexpected_bind_group_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, false), Some(bind_group(3, 2, 1)),),
            Err("draw does not accept a bind group".into())
        );
    }

    #[test]
    fn bind_group_for_a_different_draw_is_rejected() {
        assert_eq!(
            validate_render_command(DeviceKey(1), draw(2, 1, true), Some(bind_group(3, 4, 1)),),
            Err("bind group was created for a different draw".into())
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
}
