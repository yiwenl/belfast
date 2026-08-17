# Rust Experiment Template and Helpers Design

## Goal

Add reusable Rust `OrbitalControl` and `AxisHelper` APIs, then compose them in a single-file Cargo example that can be copied to start a small native Belfast experiment quickly.

The implementation must preserve the existing TypeScript package and examples. It must also keep the reusable `belfast` crate independent of `winit`, DOM APIs, and `wasm-bindgen` so the same helper logic remains usable by native Rust applications and future WebAssembly adapters.

## Scope

This milestone includes:

- A platform-independent orbital camera controller in the `belfast` crate.
- A GPU-rendered RGB axis helper in the `belfast` crate.
- Platform-neutral input events in the native example harness, translated from `winit::WindowEvent`.
- Frame delta delivery from the native example harness.
- A copyable `examples/template.rs` that uses only Belfast's public API plus the shared example harness.
- Focused unit tests and native GPU validation.

This milestone does not include:

- A general scene graph or entity/component system.
- A public, engine-wide input abstraction.
- A generic camera trait or orthographic orbital controls.
- Touch gestures, keyboard navigation, pointer capture, or controller input.
- Axis labels, arrows, grids, gizmo picking, or transform manipulation.
- A browser input adapter in `belfast-wasm`.
- A generator CLI or standalone Cargo package per experiment.

## File Structure

```text
rust/crates/belfast/
├── src/
│   ├── controls/
│   │   ├── mod.rs
│   │   └── orbital_control.rs
│   ├── helpers/
│   │   ├── axis_helper.rs
│   │   ├── axis_helper.wgsl
│   │   └── mod.rs
│   └── lib.rs
└── examples/
    ├── common/
    │   ├── input.rs
    │   └── mod.rs
    └── template.rs
```

`controls` contains CPU-only interaction state. `helpers` contains reusable renderable diagnostics. Native event-loop concerns remain in `examples/common`, which is compiled only for examples through dev-dependencies.

## Orbital Control

### Public API

The Rust API keeps the Belfast class name while following Rust naming and ownership conventions:

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

impl Default for OrbitalControlOptions { /* values specified below */ }

pub struct OrbitalControl { /* current, target, and drag state */ }

impl OrbitalControl {
    pub fn new(options: OrbitalControlOptions) -> BelfastResult<Self>;
    pub fn pointer_down(
        &mut self,
        position: [f32; 2],
        button: OrbitalPointerButton,
        pan_modifier: bool,
    );
    pub fn pointer_move(&mut self, position: [f32; 2], viewport: [f32; 2]);
    pub fn pointer_up(&mut self, button: OrbitalPointerButton);
    pub fn scroll(&mut self, delta: f32);
    pub fn update(
        &mut self,
        delta_seconds: f32,
        camera: &mut PerspectiveCamera,
    ) -> bool;
    pub fn center(&self) -> [f32; 3];
    pub fn eye(&self) -> [f32; 3];
    pub fn radius(&self) -> f32;
}
```

`OrbitalControl` does not own or borrow a camera. The application owns both values and passes its `PerspectiveCamera` to `update`. This avoids long-lived Rust borrows and allows a renderer to replace or temporarily stop updating its camera.

The constructor validates finite numeric options, `min_radius > 0`, `max_radius >= min_radius`, and `radius` within the inclusive minimum/maximum range. Invalid values return a specific `BelfastError` instead of being silently accepted.

The defaults are:

```rust
OrbitalControlOptions {
    center: [0.0, 0.0, 0.0],
    radius: 10.0,
    min_radius: 0.0001,
    max_radius: f32::MAX,
    rotate_sensitivity: 0.01,
    zoom_sensitivity: 0.002,
    pan_sensitivity: 2.0,
    damping: 12.0,
}
```

### Interaction Model

- Primary-button drag rotates around the center.
- Middle-button drag pans.
- Primary-button drag with `pan_modifier == true` pans.
- Scroll changes the target radius exponentially so zoom remains useful at both small and large scales. A positive normalized delta zooms out and a negative delta zooms in.
- Pitch is clamped just inside `-PI / 2..PI / 2` to prevent the view from crossing a pole and flipping.
- Radius is clamped to `min_radius..max_radius`.
- Pointer movement is ignored when no supported button is active.
- A viewport with a zero dimension is ignored to avoid division by zero.

The controller stores current and target values for center, yaw, pitch, and radius. `update` uses the response factor `1.0 - exp(-damping * delta_seconds)`, making damping stable across different frame rates. `damping == 0.0` means immediate response. A non-positive delta does not advance damping but still applies the current pose. Values within a small fixed epsilon of their targets snap to the targets so the controller reaches a stable state. `update` returns whether the applied pose changed, allowing consumers to render on demand later even though the native examples currently redraw continuously.

The eye position is derived with Belfast's existing right-handed, Y-up convention:

```text
x = center.x + radius * cos(pitch) * sin(yaw)
y = center.y + radius * sin(pitch)
z = center.z + radius * cos(pitch) * cos(yaw)
```

Panning moves the center along the current camera-right and camera-up vectors. World units per pixel are calculated as `radius * pan_sensitivity / viewport_height`, so the perceived movement remains useful across zoom levels.

### Platform Boundary

The core controller accepts plain numbers and Belfast enums only. It does not know about `winit::MouseButton`, `MouseScrollDelta`, browser events, CSS pixels, or device pixel ratio.

The native example adapter converts `winit` input into a small example-only enum:

```rust
pub enum InputEvent {
    PointerDown {
        position: [f32; 2],
        button: OrbitalPointerButton,
        pan_modifier: bool,
    },
    PointerMove {
        position: [f32; 2],
    },
    PointerUp {
        button: OrbitalPointerButton,
    },
    Scroll {
        delta: f32,
    },
}
```

Unsupported mouse buttons are not forwarded. The harness tracks the last cursor position and Shift modifier state because `winit::WindowEvent::MouseInput` does not carry those values. Scroll is normalized so positive values mean zoom out: pixel values retain their magnitude with the required sign conversion, while each line is converted to `16.0` pixels.

The example trait gains default no-op `input` and `update` hooks, preserving all existing examples without modification:

```rust
fn input(&mut self, _context: &ExampleContext, _event: InputEvent) {}
fn update(&mut self, _context: &ExampleContext, _delta_seconds: f32) {}
```

The harness measures frame time with `std::time::Instant`, clamps each frame delta to `0.1` seconds, and calls `update` immediately before `render`. It resets its timestamp after surface recovery so a lost or suspended surface does not produce a large control jump.

## Axis Helper

### Public API

`AxisHelper` owns immutable axis vertex buffers, a `Mesh`, and a `Draw` pipeline. It does not own camera uniforms or bind groups.

```rust
pub struct AxisHelperOptions<'a> {
    pub label: &'a str,
    pub length: f32,
    pub format: wgpu::TextureFormat,
    pub layout: &'a wgpu::PipelineLayout,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> AxisHelperOptions<'a> {
    pub fn new(
        format: wgpu::TextureFormat,
        layout: &'a wgpu::PipelineLayout,
    ) -> Self;
}

pub struct AxisHelper { /* mesh and draw */ }

impl AxisHelper {
    pub fn new(device: &Device, options: AxisHelperOptions<'_>) -> BelfastResult<Self>;
    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a BindGroup,
    );
}
```

The caller supplies a pipeline layout whose group 0 binding 0 is a vertex-stage uniform buffer containing a `mat4x4<f32>` view-projection matrix. Reusing the caller's layout guarantees that its camera bind group is compatible with the axis pipeline. The template creates one explicit camera bind group layout and uses the same pipeline layout for its scene draw and `AxisHelper`.

`AxisHelperOptions::new` defaults to length `1.0`, label `"AxisHelper"`, and no depth state. A non-finite or non-positive length returns a specific `BelfastError`.

### Geometry and Rendering

The helper renders three positive line segments from the origin:

- X: `[0, 0, 0]` to `[length, 0, 0]`, red.
- Y: `[0, 0, 0]` to `[0, length, 0]`, green.
- Z: `[0, 0, 0]` to `[0, 0, length]`, blue.

Positions and colors use separate vertex buffers, matching the existing Belfast mesh API and the `colored_triangle` example. The primitive topology is `LineList`. The WGSL shader lives next to the helper implementation and is embedded with `include_str!`, keeping the public API self-contained for downstream crates.

The default has no depth attachment so it works with the existing surface-pass helper. Callers with a depth buffer may provide a matching `DepthStencilState`; render-pass compatibility remains the caller's responsibility, as it is for `DrawOptions`.

## Experiment Template

`rust/crates/belfast/examples/template.rs` is a normal Cargo example. A new experiment starts with:

```bash
cp crates/belfast/examples/template.rs crates/belfast/examples/my_experiment.rs
cargo run -p belfast --example my_experiment
```

The copied file remains intentionally self-contained. Its WGSL is an inline constant so one file is sufficient to copy. Shared windowing and input translation stay in `examples/common`, just like the existing examples.

The template demonstrates:

- Position and color vertex buffers for a small triangle.
- An explicit camera bind group layout and pipeline layout.
- A `PerspectiveCamera`, `UniformBlock`, and camera `BindGroup`.
- `OrbitalControl` event handling and per-frame camera updates.
- `AxisHelper` rendered in the same pass.
- Aspect-ratio and camera-uniform updates after resize.
- Clearly separated `new`, `input`, `update`, and `render` responsibilities.

The template is implementation-oriented and contains only short comments that identify safe customization points. It does not include tutorial prose or duplicate the engine's internal abstractions.

## Data Flow

```text
winit WindowEvent
    -> examples/common input adapter
    -> Example::input(InputEvent)
    -> OrbitalControl pointer/scroll methods
    -> Example::update(delta_seconds)
    -> OrbitalControl::update(camera)
    -> camera view-projection matrix
    -> UniformBlock upload
    -> scene Draw + AxisHelper::draw
```

Resize follows a separate path: the harness updates `ExampleContext`, the template updates camera aspect, and the next update uploads the revised view-projection matrix.

## Error Handling

- Invalid controller options and invalid axis length use new typed `BelfastError` variants.
- Runtime pointer and scroll input that is unsupported or non-finite is ignored rather than poisoning camera state.
- Zero-sized viewports are ignored by control movement and already suppressed by the native surface harness.
- GPU pipeline creation follows current `wgpu` validation behavior and existing Belfast constructor conventions.

## Testing and Verification

CPU unit tests cover:

- Orbital option validation.
- Initial eye and center values.
- Rotation direction and pitch clamping.
- Pan behavior and zero-sized viewport handling.
- Exponential zoom and radius clamping.
- Damping consistency for equivalent elapsed time split across different frame steps.
- Ignoring non-finite input.
- Axis vertex positions, colors, and length validation through a pure geometry builder.

Build and lint verification covers:

```bash
cd rust
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
```

Native validation launches `template` long enough to confirm adapter setup, shader compilation, bind-group compatibility, line-list pipeline creation, and rendering without a `wgpu` validation error:

```bash
cargo run -p belfast --example template
```

Existing examples must continue to compile and run without implementing the new default trait hooks.

## Compatibility

- Existing TypeScript source and examples are unchanged.
- Existing Rust public APIs remain source-compatible; the helper APIs are additive.
- `winit` remains a dev-dependency of `belfast`, not a library dependency.
- `belfast-wasm` continues to compile without immediately exposing the new helpers.
- The single-file template follows Cargo's existing example discovery and needs no manifest entry or generator command.
