# Rust Experiment Template and Helpers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add platform-independent orbital camera controls, a reusable RGB axis renderer, and a copyable single-file Rust experiment example.

**Architecture:** `OrbitalControl` remains CPU-only and receives normalized pointer input without depending on a window framework. `AxisHelper` owns Belfast mesh and draw resources but reuses the caller's camera bind group and pipeline layout. The native example harness alone translates `winit` events and supplies frame deltas, while `template.rs` demonstrates composition through public Belfast APIs.

**Tech Stack:** Rust 2021, `wgpu` 0.20, `winit` 0.30 as a dev-dependency, `glam` 0.30, WGSL, Cargo integration tests.

## Global Constraints

- Keep all new Rust implementation under `rust/`; use the existing `docs/superpowers/` area only for design and plan documents.
- Do not modify the TypeScript Belfast package or TypeScript examples.
- Do not add `winit`, `wasm-bindgen`, `web-sys`, or DOM dependencies to the reusable `belfast` crate dependencies.
- Keep `winit` as a `dev-dependency` used only by native examples.
- Keep the template as one copyable file at `rust/crates/belfast/examples/template.rs`.
- Preserve existing Rust APIs and examples through additive APIs and default trait methods, except for the explicitly accepted one-time pre-1.0 `#[non_exhaustive]` change to `BelfastError`.
- Use Belfast's right-handed, Y-up camera convention.
- Do not expose the helpers from `belfast-wasm` in this milestone, but keep the WASM crate compiling.

---

### Task 1: Platform-Independent Orbital Control

**Files:**

- Create: `rust/crates/belfast/src/controls/mod.rs`
- Create: `rust/crates/belfast/src/controls/orbital_control.rs`
- Create: `rust/crates/belfast/tests/orbital_control.rs`
- Modify: `rust/crates/belfast/src/error.rs`
- Modify: `rust/crates/belfast/src/lib.rs`

**Interfaces:**

- Consumes: `PerspectiveCamera::look_at(&mut self, [f32; 3], [f32; 3]) -> &mut Self` and `glam::Vec3`.
- Produces: crate-root exports `OrbitalControl`, `OrbitalControlOptions`, and `OrbitalPointerButton`.
- Produces: `BelfastError::InvalidOrbitalControlOption(&'static str)`.

- [ ] **Step 1: Write failing tests for defaults and validation**

Create `rust/crates/belfast/tests/orbital_control.rs`:

```rust
use belfast::{
    BelfastError, OrbitalControl, OrbitalControlOptions, OrbitalPointerButton,
    PerspectiveCamera,
};

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "expected {expected}, got {actual}");
}

fn camera() -> PerspectiveCamera {
    PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0)
}

#[test]
fn orbital_control_defaults_to_positive_z() {
    let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
    let mut camera = camera();
    control.update(0.0, &mut camera);

    assert_eq!(control.center(), [0.0, 0.0, 0.0]);
    assert_eq!(control.eye(), [0.0, 0.0, 10.0]);
    assert_eq!(camera.look_at_target(), [0.0, 0.0, 0.0]);
}

#[test]
fn orbital_control_rejects_invalid_radius_range() {
    let result = OrbitalControl::new(OrbitalControlOptions {
        min_radius: 2.0,
        max_radius: 1.0,
        ..Default::default()
    });

    assert!(matches!(
        result,
        Err(BelfastError::InvalidOrbitalControlOption("max_radius"))
    ));
}
```

- [ ] **Step 2: Run the test and verify the missing API failure**

Run: `cd rust && cargo test -p belfast --test orbital_control`

Expected: compilation fails because the three orbital types and error variant are undefined.

- [ ] **Step 3: Add options, state, validation, and camera application**

Add to `BelfastError`:

```rust
#[non_exhaustive]
pub enum BelfastError {
    #[error("orbital control option `{0}` is invalid")]
    InvalidOrbitalControlOption(&'static str),
}
```

The public enum becomes non-exhaustive so Belfast can keep typed helper errors and add future variants without another downstream exhaustive-match break.

Create `controls/mod.rs`:

```rust
mod orbital_control;

pub use orbital_control::{OrbitalControl, OrbitalControlOptions, OrbitalPointerButton};
```

Define the public API and defaults in `orbital_control.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbitalPointerButton {
    Primary,
    Middle,
}

#[derive(Clone, Copy, Debug)]
pub struct OrbitalControlOptions {
    pub center: [f32; 3],
    pub radius: f32,
    pub min_radius: f32,
    pub max_radius: f32,
    pub rotate_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub damping: f32,
}

impl Default for OrbitalControlOptions {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0, 0.0],
            radius: 10.0,
            min_radius: 0.0001,
            max_radius: f32::MAX,
            rotate_sensitivity: 0.01,
            zoom_sensitivity: 0.002,
            pan_sensitivity: 2.0,
            damping: 12.0,
        }
    }
}
```

Use private `DragMode::{Rotate, Pan}` variants that retain the initiating button, pointer origin, and starting target values. Store current and target center, radius, yaw, and pitch in `OrbitalControl`; initialize each current value equal to its target.

Validate every option with `is_finite`, require `min_radius > 0.0`, require `max_radius >= min_radius`, require `radius` inside the inclusive minimum/maximum range, and reject negative sensitivities or damping. Return the exact offending field name through `InvalidOrbitalControlOption`, including for each negative sensitivity.

Use these functions for pose and frame-rate-independent interpolation:

```rust
const SNAP_EPSILON: f32 = 0.00001;

fn eye_from_pose(center: Vec3, radius: f32, yaw: f32, pitch: f32) -> Vec3 {
    let horizontal_radius = pitch.cos() * radius;
    center + Vec3::new(
        yaw.sin() * horizontal_radius,
        pitch.sin() * radius,
        yaw.cos() * horizontal_radius,
    )
}

fn response(damping: f32, delta_seconds: f32) -> f32 {
    if damping == 0.0 { 1.0 } else { 1.0 - (-damping * delta_seconds.max(0.0)).exp() }
}

fn interpolate(value: f32, target: f32, interpolation: f32) -> f32 {
    let candidate = value as f64
        + (target as f64 - value as f64) * interpolation as f64;
    f32_from_f64(candidate).unwrap_or(value)
}

fn snap(value: f32, target: f32) -> f32 {
    if (value as f64 - target as f64).abs() <= SNAP_EPSILON as f64 {
        target
    } else {
        value
    }
}
```

`f32_from_f64` returns `Some(value as f32)` only for finite values within `f32::MAX`; otherwise it returns `None`. `update` interpolates every current component with `interpolate`, snaps near-target values, derives the eye, calls `camera.look_at`, and returns whether current pose values changed. Add exact getters `center() -> [f32; 3]`, `eye() -> [f32; 3]`, and `radius() -> f32`. Declare `mod controls;` and re-export all three types from `lib.rs`.

- [ ] **Step 4: Run the initial tests and verify they pass**

Run: `cd rust && cargo test -p belfast --test orbital_control`

Expected: both tests pass.

- [ ] **Step 5: Add failing interaction and damping tests**

Append these cases to `tests/orbital_control.rs`:

```rust
#[test]
fn primary_drag_rotates_toward_negative_x_and_clamps_pitch() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    }).unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([100.0, 100_000.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.eye()[0] < 0.0);
    assert!(control.eye()[1] > 0.99 * control.radius());
    assert!(control.eye()[1] < control.radius());
}

#[test]
fn shift_primary_drag_pans_the_target() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    }).unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, true);
    control.pointer_move([60.0, 30.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.center()[0] < 0.0);
    assert!(control.center()[1] > 0.0);
    assert_eq!(camera.look_at_target(), control.center());
}

#[test]
fn scroll_clamps_radius_and_ignores_non_finite_input() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        radius: 2.0,
        min_radius: 1.0,
        max_radius: 3.0,
        damping: 0.0,
        ..Default::default()
    }).unwrap();
    let mut camera = camera();
    control.scroll(-100_000.0);
    control.update(1.0 / 60.0, &mut camera);
    assert_approx_eq(control.radius(), 1.0);

    control.scroll(100_000.0);
    control.scroll(f32::NAN);
    control.update(1.0 / 60.0, &mut camera);
    assert_approx_eq(control.radius(), 3.0);
}

#[test]
fn damping_depends_on_elapsed_time_not_frame_count() {
    fn simulate(step: f32, count: usize) -> [f32; 3] {
        let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
        let mut camera = camera();
        control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
        control.pointer_move([80.0, 20.0], [800.0, 600.0]);
        for _ in 0..count { control.update(step, &mut camera); }
        control.eye()
    }

    let sixty_hz = simulate(1.0 / 60.0, 60);
    let thirty_hz = simulate(1.0 / 30.0, 30);
    for index in 0..3 { assert_approx_eq(sixty_hz[index], thirty_hz[index]); }
}
```

- [ ] **Step 6: Implement rotate, pan, scroll, and input guards**

Use absolute pointer displacement from the drag origin:

```rust
let horizontal_displacement = position[0] as f64 - start[0] as f64;
let vertical_displacement = position[1] as f64 - start[1] as f64;
let sensitivity = self.rotate_sensitivity as f64;
self.target_yaw = normalize_yaw(
    start_yaw as f64 - horizontal_displacement * sensitivity,
) as f32;
self.target_pitch = (start_pitch as f64 + vertical_displacement * sensitivity)
    .clamp(-pitch_limit, pitch_limit) as f32;
```

This reversed horizontal direction means a primary drag from `[0.0, 0.0]` to `[100.0, 0.0]` places the eye at negative X after an immediate update. Normalize yaw to an equivalent value in `[-PI, PI]` before converting to `f32`, and clamp pitch with `f64` intermediates.

For panning, derive camera-right and camera-up from the drag's starting yaw and pitch, then build the target with `f64` arithmetic:

```rust
let yaw = start_yaw as f64;
let pitch = start_pitch as f64;
let right = [yaw.cos(), 0.0, -yaw.sin()];
let camera_up = [
    -yaw.sin() * pitch.sin(),
    pitch.cos(),
    -yaw.cos() * pitch.sin(),
];
let horizontal_displacement = position[0] as f64 - start[0] as f64;
let vertical_displacement = position[1] as f64 - start[1] as f64;
let scale = self.target_radius as f64 * self.pan_sensitivity as f64
    / viewport[1] as f64;
let start_center = start_center.to_array().map(f64::from);
let candidate = std::array::from_fn(|index| {
    start_center[index] - right[index] * horizontal_displacement * scale
        + camera_up[index] * vertical_displacement * scale
});
if let Some(candidate) = vec3_from_f64(candidate) {
    self.target_center = candidate;
}
```

For scroll, use:

```rust
let exponent = (delta * self.zoom_sensitivity).clamp(-80.0, 80.0);
self.target_radius = (self.target_radius * exponent.exp())
    .clamp(self.min_radius, self.max_radius);
```

Ignore non-finite pointer, viewport, and scroll values. Ignore zero-sized viewports and movement without an active drag. `pointer_up` clears only a drag initiated by the same button. Perform drag and interpolation subtraction/multiplication in `f64`; keep yaw bounded, and preserve current finite state whenever a derived candidate is non-finite or outside the `f32` range.

Add focused regressions before this hardening implementation for maximum-finite rotation sensitivity with a two-pixel drag, maximum-finite pan sensitivity, interpolation between opposite `f32` extremes, active-drag non-finite movement, active zero-width/zero-height viewports, exact table-driven invalid options, moderate formula-based exponential zoom, and all zero/negative/non-finite axis lengths. The extreme drag tests assert finite center, eye, and camera view-projection output.

- [ ] **Step 7: Run tests and commit the controller**

Run:

```bash
cd rust
cargo test -p belfast --test orbital_control
cargo test -p belfast --all-targets
```

Expected: all tests pass.

Commit:

```bash
git add rust/crates/belfast/src/controls rust/crates/belfast/src/error.rs rust/crates/belfast/src/lib.rs rust/crates/belfast/tests/orbital_control.rs
git commit -m "feat(rust): add orbital camera control"
```

---

### Task 2: RGB Axis Helper

**Files:**

- Create: `rust/crates/belfast/src/helpers/mod.rs`
- Create: `rust/crates/belfast/src/helpers/axis_helper.rs`
- Create: `rust/crates/belfast/src/helpers/axis_helper.wgsl`
- Create: `rust/crates/belfast/tests/axis_helper.rs`
- Modify: `rust/crates/belfast/src/error.rs`
- Modify: `rust/crates/belfast/src/lib.rs`
- Modify: `rust/crates/belfast/tests/example_shaders.rs`

**Interfaces:**

- Consumes: `Device`, `Buffer`, `Mesh`, `Draw`, `BindGroup`, and a caller-owned `wgpu::PipelineLayout` whose group 0 binding 0 is a vertex-stage `mat4x4<f32>` uniform.
- Produces: crate-root exports `AxisHelper` and `AxisHelperOptions<'a>`.
- Produces: `BelfastError::InvalidAxisLength`.

- [ ] **Step 1: Write failing validation and GPU construction tests**

Create `tests/axis_helper.rs` with the optional headless-device pattern used by `tests/render_target.rs` and this layout helper:

```rust
fn camera_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
    device.gpu().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("AxisTestCameraLayout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}
```

Add these tests:

```rust
#[test]
fn axis_helper_rejects_non_positive_length() {
    let Some(device) = create_optional_device() else { return; };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device.gpu().create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("AxisTestPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        },
    );
    let result = AxisHelper::new(
        &device,
        AxisHelperOptions {
            length: 0.0,
            ..AxisHelperOptions::new(device.format(), &pipeline_layout)
        },
    );

    assert!(matches!(result, Err(BelfastError::InvalidAxisLength)));
}

#[test]
fn axis_helper_builds_with_a_compatible_camera_layout() {
    let Some(device) = create_optional_device() else { return; };
    let bind_group_layout = camera_bind_group_layout(&device);
    let pipeline_layout = device.gpu().create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("AxisTestPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        },
    );

    device.gpu().push_error_scope(wgpu::ErrorFilter::Validation);
    let helper = AxisHelper::new(
        &device,
        AxisHelperOptions::new(device.format(), &pipeline_layout),
    );
    device.gpu().poll(wgpu::Maintain::Wait);
    let error = pollster::block_on(device.gpu().pop_error_scope());

    assert!(helper.is_ok());
    assert!(error.is_none(), "{}", error.unwrap());
}
```

- [ ] **Step 2: Run the test and verify the missing API failure**

Run: `cd rust && cargo test -p belfast --test axis_helper`

Expected: compilation fails because `AxisHelper`, `AxisHelperOptions`, and `InvalidAxisLength` are undefined.

- [ ] **Step 3: Add the pure geometry builder and its unit test**

Start `helpers/axis_helper.rs` with:

```rust
fn axis_geometry(length: f32) -> ([f32; 18], [f32; 18]) {
    (
        [
            0.0, 0.0, 0.0, length, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, length, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, length,
        ],
        [
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::axis_geometry;

    #[test]
    fn geometry_contains_three_colored_positive_axes() {
        let (positions, colors) = axis_geometry(2.0);
        assert_eq!(&positions[3..6], &[2.0, 0.0, 0.0]);
        assert_eq!(&positions[9..12], &[0.0, 2.0, 0.0]);
        assert_eq!(&positions[15..18], &[0.0, 0.0, 2.0]);
        assert_eq!(&colors[0..6], &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(&colors[12..18], &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    }
}
```

- [ ] **Step 4: Implement the helper shader and GPU resources**

Add `BelfastError::InvalidAxisLength` with message `"axis length must be finite and greater than 0"`.

Create `helpers/axis_helper.wgsl`:

```wgsl
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(position, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
```

Implement these exact public signatures:

```rust
pub struct AxisHelperOptions<'a> {
    pub label: &'a str,
    pub length: f32,
    pub format: wgpu::TextureFormat,
    pub layout: &'a wgpu::PipelineLayout,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> AxisHelperOptions<'a> {
    pub fn new(format: wgpu::TextureFormat, layout: &'a wgpu::PipelineLayout) -> Self;
}

pub struct AxisHelper {
    mesh: Mesh,
    draw: Draw,
}

impl AxisHelper {
    pub fn new(device: &Device, options: AxisHelperOptions<'_>) -> BelfastResult<Self>;
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a BindGroup);
}
```

Default options are label `"AxisHelper"`, length `1.0`, supplied format/layout, and no depth state. Reject non-finite or non-positive length. Create separate position and color `Buffer::from_data` buffers, then a six-vertex `Mesh` with `Float32x3` attributes at locations 0 and 1. Configure the pipeline with:

```rust
let mut draw_options = DrawOptions::new(options.label, options.format);
draw_options.layout = Some(options.layout);
draw_options.primitive.topology = wgpu::PrimitiveTopology::LineList;
draw_options.depth_stencil = options.depth_stencil;
```

Embed the shader with `include_str!("axis_helper.wgsl")`. The draw method binds the supplied camera group at index 0 and renders one instance. Add `helpers/mod.rs`, declare `mod helpers;`, and re-export both public helper types from `lib.rs`.

- [ ] **Step 5: Add the helper shader to standalone validation**

Append to the shader list in `tests/example_shaders.rs`:

```rust
(
    "axis_helper",
    include_str!("../src/helpers/axis_helper.wgsl"),
),
```

- [ ] **Step 6: Run tests and commit the helper**

Run:

```bash
cd rust
cargo test -p belfast --test axis_helper
cargo test -p belfast --test example_shaders
cargo test -p belfast --all-targets
```

Expected: geometry, validation, GPU construction, shader validation, and existing tests pass.

Commit:

```bash
git add rust/crates/belfast/src/helpers rust/crates/belfast/src/error.rs rust/crates/belfast/src/lib.rs rust/crates/belfast/tests/axis_helper.rs rust/crates/belfast/tests/example_shaders.rs
git commit -m "feat(rust): add axis helper"
```

---

### Task 3: Native Input and Frame Timing Harness

**Files:**

- Create: `rust/crates/belfast/examples/common/input.rs`
- Modify: `rust/crates/belfast/examples/common/mod.rs`

**Interfaces:**

- Consumes: `winit::event::WindowEvent`, mouse events, and modifiers from the existing dev-dependency.
- Produces: example-only `InputEvent` and `InputState::process(&WindowEvent) -> Option<InputEvent>`.
- Produces: default `Example::input` and `Example::update` hooks.

- [ ] **Step 1: Wire a test-only input module and write failing mapping tests**

Create `examples/common/input.rs` with only these tests:

```rust
#[cfg(test)]
mod tests {
    use super::{map_button, normalize_scroll};
    use belfast::OrbitalPointerButton;
    use winit::{
        dpi::PhysicalPosition,
        event::{MouseButton, MouseScrollDelta},
    };

    #[test]
    fn maps_only_supported_pointer_buttons() {
        assert_eq!(map_button(MouseButton::Left), Some(OrbitalPointerButton::Primary));
        assert_eq!(map_button(MouseButton::Middle), Some(OrbitalPointerButton::Middle));
        assert_eq!(map_button(MouseButton::Right), None);
    }

    #[test]
    fn normalizes_scroll_to_positive_zoom_out_pixels() {
        assert_eq!(normalize_scroll(MouseScrollDelta::LineDelta(0.0, 2.0)), -32.0);
        assert_eq!(
            normalize_scroll(MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -24.0))),
            24.0
        );
    }
}
```

Declare `mod input;` in `common/mod.rs` so Cargo compiles this module for each example test target.

- [ ] **Step 2: Run example tests and verify the missing-function failure**

Run: `cd rust && cargo test -p belfast --examples`

Expected: compilation fails because `map_button` and `normalize_scroll` are not defined.

- [ ] **Step 3: Implement input state and event translation**

Add the production imports, constant, and mapping functions above the tests in `input.rs`:

```rust
use belfast::OrbitalPointerButton;
use winit::event::{MouseButton, MouseScrollDelta};

const LINE_SCROLL_PIXELS: f32 = 16.0;

fn map_button(button: MouseButton) -> Option<OrbitalPointerButton> {
    match button {
        MouseButton::Left => Some(OrbitalPointerButton::Primary),
        MouseButton::Middle => Some(OrbitalPointerButton::Middle),
        _ => None,
    }
}

fn normalize_scroll(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => -y * LINE_SCROLL_PIXELS,
        MouseScrollDelta::PixelDelta(position) => -position.y as f32,
    }
}
```

Then add:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    PointerDown {
        position: [f32; 2],
        button: OrbitalPointerButton,
        pan_modifier: bool,
    },
    PointerMove { position: [f32; 2] },
    PointerUp { button: OrbitalPointerButton },
    Scroll { delta: f32 },
}

#[derive(Default)]
pub struct InputState {
    cursor_position: [f32; 2],
    shift_pressed: bool,
}
```

Implement `InputState::process(&mut self, event: &WindowEvent) -> Option<InputEvent>` with these exact mappings:

- `CursorMoved`: save physical `x/y` as `f32`, emit `PointerMove`.
- `ModifiersChanged`: save `modifiers.state().shift_key()`, emit nothing.
- Supported `MouseInput` pressed: emit `PointerDown` with saved cursor and Shift state.
- Supported `MouseInput` released: emit `PointerUp`.
- `MouseWheel`: emit `Scroll` using `normalize_scroll`.
- Every other event and unsupported mouse button: return `None`.

Add `pub use input::InputEvent;` in `common/mod.rs` now that the event type exists.

- [ ] **Step 4: Add default example hooks and dispatch input**

Extend `Example`:

```rust
fn input(&mut self, _context: &ExampleContext, _event: InputEvent) {}

fn update(&mut self, _context: &ExampleContext, _delta_seconds: f32) {}
```

Add `input: input::InputState` to `ExampleState` and initialize it with `Default::default()`. Before matching the owned `WindowEvent`, dispatch:

```rust
if let Some(input_event) = state.input.process(&event) {
    state.example.input(&state.context, input_event);
}
```

- [ ] **Step 5: Add bounded frame timing**

Import `std::time::Instant`, add `last_frame_at: Instant` to `ExampleState`, and initialize it after constructing the example. At the start of `render`, calculate:

```rust
let now = Instant::now();
let delta_seconds = (now - self.last_frame_at).as_secs_f32().min(0.1);
self.last_frame_at = now;
```

After successfully acquiring the surface frame and before `example.render`, call:

```rust
self.example.update(&self.context, delta_seconds);
```

Reset `last_frame_at = Instant::now()` in the `Lost | Outdated` recovery branch. Keep continuous `request_redraw` behavior unchanged.

- [ ] **Step 6: Run harness tests and compile every example**

Run:

```bash
cd rust
cargo test -p belfast --examples
cargo check -p belfast --examples
```

Expected: mapping tests pass and all existing examples compile unchanged because the new hooks have defaults.

- [ ] **Step 7: Commit the harness extension**

```bash
git add rust/crates/belfast/examples/common
git commit -m "feat(rust): add example input hooks"
```

---

### Task 4: Copyable Experiment Template and Final Verification

**Files:**

- Create: `rust/crates/belfast/examples/template.rs`
- Modify: `rust/README.md`
- Verify: `docs/superpowers/specs/2026-08-18-rust-experiment-template-helpers-design.md`
- Verify: `docs/superpowers/plans/2026-08-18-rust-experiment-template-helpers.md`

**Interfaces:**

- Consumes: all public helper APIs from Tasks 1 and 2 and `common::{Example, ExampleContext, InputEvent}` from Task 3.
- Produces: Cargo example target `template`, runnable with `cargo run -p belfast --example template` and copyable without editing a manifest.

- [ ] **Step 1: Create the single-file template state and shader**

Create `examples/template.rs` with `mod common;` and this state:

```rust
struct Experiment {
    mesh: Mesh,
    draw: Draw,
    axis: AxisHelper,
    camera: PerspectiveCamera,
    control: OrbitalControl,
    camera_uniform: UniformBlock,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
}
```

Add this inline shader and use a three-vertex XY-plane triangle with separate position/color buffers matching `camera_uniform.rs`:

```rust
const SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(position, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;
```

- [ ] **Step 2: Build one camera layout shared by both pipelines**

In `Experiment::new`, create a bind group layout containing:

```rust
wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStages::VERTEX,
    ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
}
```

Create a pipeline layout from it. Set `DrawOptions::layout = Some(&pipeline_layout)` for the triangle and construct the helper with:

```rust
AxisHelper::new(
    &context.device,
    AxisHelperOptions {
        length: 1.5,
        ..AxisHelperOptions::new(context.format, &pipeline_layout)
    },
)
.expect("axis helper")
```

Create `UniformBlock::create([("view_proj", UniformFieldType::Mat4x4F)])`, its uniform buffer, and a bind group from the explicit camera bind group layout.

Initialize camera and controls with:

```rust
let mut camera = PerspectiveCamera::new(
    std::f32::consts::FRAC_PI_4,
    context.width as f32 / context.height as f32,
    0.1,
    100.0,
);
let mut control = OrbitalControl::new(OrbitalControlOptions {
    radius: 4.0,
    ..Default::default()
})
.expect("valid orbital control");
control.update(0.0, &mut camera);
```

- [ ] **Step 3: Connect input, resize, and frame updates**

Implement the input hook without importing `winit`:

```rust
fn input(&mut self, context: &ExampleContext, event: InputEvent) {
    match event {
        InputEvent::PointerDown { position, button, pan_modifier } => {
            self.control.pointer_down(position, button, pan_modifier);
        }
        InputEvent::PointerMove { position } => {
            self.control.pointer_move(
                position,
                [context.width as f32, context.height as f32],
            );
        }
        InputEvent::PointerUp { button } => self.control.pointer_up(button),
        InputEvent::Scroll { delta } => self.control.scroll(delta),
    }
}
```

On resize, call `camera.set_aspect(context.width as f32 / context.height as f32)`. In `update`, call `control.update(delta_seconds, &mut camera)`, set `view_proj` from `camera.view_projection_matrix()`, then write `camera_uniform.bytes()` to `camera_buffer`.

- [ ] **Step 4: Render the experiment and axes in one pass**

In `render`, create one command encoder and one `common::begin_render_pass`. Bind the camera group at index 0, draw the triangle, then render the axes:

```rust
self.camera_bind_group.bind(&mut pass, 0);
self.draw.draw(&mut pass, &self.mesh, 1);
self.axis.draw(&mut pass, &self.camera_bind_group);
```

Submit the encoder once and add:

```rust
fn main() {
    common::run::<Experiment>("Belfast Rust - Experiment Template");
}
```

- [ ] **Step 5: Compile the template and all test targets**

Run:

```bash
cd rust
cargo check -p belfast --example template
cargo test --workspace --all-targets
```

Expected: the template and all existing unit, integration, library, WASM host, and example targets pass.

- [ ] **Step 6: Document the copy workflow**

Add `cargo run -p belfast --example template` to the native example list in `rust/README.md`, followed by:

````markdown
Start a small experiment by copying the template from the `rust` directory:

```bash
cp crates/belfast/examples/template.rs crates/belfast/examples/my_experiment.rs
cargo run -p belfast --example my_experiment
```
````

- [ ] **Step 7: Run final automated and native verification**

Run from `rust/`:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
cargo run -p belfast --example template
```

Expected: formatting and lint are clean, all tests pass, WASM compiles, and the native window shows the colored triangle plus RGB axes. Verify orbit, Shift-primary pan, middle-button pan, and wheel zoom without a `wgpu` validation error; then close the window.

- [ ] **Step 8: Review scope and commit the template and documents**

Run:

```bash
git diff --check
git status --short
git diff -- packages examples
```

Expected: no whitespace errors; only planned Rust, README, spec, and plan files changed; the last command prints no TypeScript package or TypeScript example diff.

Commit:

```bash
git add rust/crates/belfast/examples/template.rs rust/README.md docs/superpowers/specs/2026-08-18-rust-experiment-template-helpers-design.md docs/superpowers/plans/2026-08-18-rust-experiment-template-helpers.md
git commit -m "feat(rust): add experiment template"
```
