# Rust WASM Texture And Render Target Examples Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable Rust-owned texture sampling and multi-pass frame APIs to `belfast-wasm`, then prove them with aspect-correct image and split color/grayscale render-to-texture browser examples.

**Architecture:** Browser image decoding stays in JavaScript and transfers an `ImageBitmap` through wgpu's external-image copy path. WASM `Draw` owns its `Mesh`; `Frame` records a linear list of logical render passes selected by `bindTarget(RenderTarget | null)` and materializes them during `submit()`. Typed `BindGroup` factories bridge sampled textures and render targets without exposing raw WebGPU handles.

**Tech Stack:** Rust 2021, wgpu 29.0.3, naga 29.0.1, wasm-bindgen, web-sys, TypeScript 5.7, Vite 6, wasm-pack.

**Spec:** `docs/superpowers/specs/2026-08-19-rust-wasm-texture-render-target-examples-design.md`

## Global Constraints

- Preserve the original TypeScript Belfast library and all native Rust APIs.
- Keep browser-only `ImageBitmap` types inside `belfast-wasm`; the core `belfast` crate remains native-compatible.
- Keep `Device.render(draw)` as the single-pass convenience API.
- WASM `Draw` consumes and owns its `Mesh`; mesh replacement is only through `Draw.setMesh(mesh)` with identical device and layout signature.
- `Frame.bindTarget(null)` and `Frame.bindTarget()` select the canvas; a `RenderTarget` selects an offscreen target.
- Only group `0` sampled `texture_2d<f32>` plus a filtering sampler are supported in this milestone.
- Reject unknown JavaScript descriptor fields with synchronous JavaScript errors.
- Every GPU resource validates creator-device and draw-layout compatibility before command recording.
- The committed image asset is exactly 1400 x 1980 JPEG quality 88; do not commit the 32 MB source PNG.
- Follow TDD: demonstrate each new validation test failing before implementing it.
- Do not commit generated `rust/pkg`, `rust/target`, `rust/web/examples/dist`, or `node_modules` output.

---

## File Structure

### Core Rust

- `rust/crates/belfast/src/texture.rs`: reusable empty 2D texture construction, resource access, and creator-device identity.
- `rust/crates/belfast/src/render_target.rs`: creator-device access used by WASM validation.
- `rust/crates/belfast/src/bind_group.rs`: creator-device identity for bound resources.
- `rust/crates/belfast/src/error.rs`: exact device/texture compatibility errors.
- `rust/crates/belfast/tests/texture.rs`: texture creation and identity tests.
- `rust/crates/belfast/tests/render_target.rs`: render-target identity and resize tests.

### WASM Facade

- `rust/crates/belfast-wasm/src/draw.rs`: owned mesh state, mesh replacement, and supported texture/sampler shader-layout validation.
- `rust/crates/belfast-wasm/src/texture.rs`: `ImageBitmap` upload and texture option conversion.
- `rust/crates/belfast-wasm/src/bind_group.rs`: typed texture/render-target bind-group factories and compatibility metadata.
- `rust/crates/belfast-wasm/src/render_target.rs`: JS-facing render target creation, resize, dimensions, and shared state.
- `rust/crates/belfast-wasm/src/frame.rs`: logical pass command list, target binding, draw recording, and submission.
- `rust/crates/belfast-wasm/src/device.rs`: frame acquisition, surface recovery, and `Device.render(draw)` convenience path.
- `rust/crates/belfast-wasm/src/resources.rs`: consuming `WasmMesh::into_inner()` bridge.
- `rust/crates/belfast-wasm/src/lib.rs`: exports for all new WASM classes.
- `rust/crates/belfast-wasm/Cargo.toml`: `ImageBitmap` web-sys feature.

### Browser Examples

- `rust/web/examples/public/scattered003.jpg`: optimized source image.
- `rust/web/examples/src/examples/texture.ts`: JavaScript fetch/decode, aspect-fit geometry, and sampled texture rendering.
- `rust/web/examples/src/examples/render-to-texture.ts`: offscreen triangle and split color/grayscale presentation.
- `rust/web/examples/src/shaders/texture.wgsl`: textured quad shader.
- `rust/web/examples/src/shaders/render-target-present.wgsl`: fullscreen split post-process shader.
- `rust/web/examples/src/main.ts`: example registration.
- `rust/README.md`: commands and query-string URLs.
- `rust/docs/rust-wgpu-api-parity.md`: partial WASM API coverage.

---

### Task 1: Core Texture And Resource Identity

**Files:**

- Modify: `rust/crates/belfast/src/texture.rs`
- Modify: `rust/crates/belfast/src/render_target.rs`
- Modify: `rust/crates/belfast/src/bind_group.rs`
- Modify: `rust/crates/belfast/src/error.rs`
- Test: `rust/crates/belfast/tests/texture.rs`
- Test: `rust/crates/belfast/tests/render_target.rs`

**Interfaces:**

- Consumes: existing `Device::is_same`, `TextureOptions`, `RenderTarget`, and `BindGroup`.
- Produces: `Texture::create_2d`, `Texture::gpu`, `Texture::device`, `RenderTarget::device`, and `BindGroup::device` for later WASM wrappers.

- [ ] **Step 1: Write failing core resource tests**

Add tests which create two headless devices and assert stable ownership:

```rust
#[test]
fn texture_tracks_creator_device() {
    let first = test_device();
    let second = test_device();
    let texture = Texture::create_2d(&first, 4, 8, TextureOptions::default()).unwrap();

    assert!(texture.device().is_same(&first));
    assert!(!texture.device().is_same(&second));
    assert_eq!(texture.width(), 4);
    assert_eq!(texture.height(), 8);
}

#[test]
fn render_target_tracks_creator_device_after_resize() {
    let device = test_device();
    let mut target = RenderTarget::create(
        &device,
        RenderTargetOptions { width: 8, height: 8, ..Default::default() },
    );
    target.resize(16, 12);

    assert!(target.device().is_same(&device));
    assert_eq!((target.width(), target.height()), (16, 12));
}
```

- [ ] **Step 2: Run the focused tests and confirm RED**

Run:

```bash
cd rust
cargo test -p belfast --test texture texture_tracks_creator_device
cargo test -p belfast --test render_target render_target_tracks_creator_device_after_resize
```

Expected: compilation fails because the new constructors/accessors do not exist.

- [ ] **Step 3: Implement reusable texture creation and identity**

Refactor `Texture::from_rgba8` to call this constructor before uploading bytes:

```rust
pub fn create_2d(
    device: &Device,
    width: u32,
    height: u32,
    options: TextureOptions,
) -> BelfastResult<Self>;

pub fn gpu(&self) -> &wgpu::Texture;
pub fn device(&self) -> &Device;
```

Store `device: Device` in `Texture`. Validate positive dimensions and
`max_texture_dimension_2d` before `create_texture`; add this exact core error:

```rust
#[error("texture dimensions {width}x{height} exceed device limit {limit}")]
TextureDimensionsExceedLimit { width: u32, height: u32, limit: u32 },
```

Ensure external image destinations can request
`TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT` through `TextureOptions.usage`.
Add `device()` accessors to `RenderTarget` and `BindGroup`; store a cloned
`Device` in `BindGroup::create`.

- [ ] **Step 4: Run core tests and workspace clippy**

Run:

```bash
cd rust
cargo test -p belfast --test texture
cargo test -p belfast --test render_target
cargo clippy -p belfast --all-targets -- -D warnings
```

Expected: all commands pass.

- [ ] **Step 5: Commit core resource support**

```bash
git add rust/crates/belfast/src rust/crates/belfast/tests/texture.rs rust/crates/belfast/tests/render_target.rs
git commit -m "feat(rust): expose texture resource identity"
```

---

### Task 2: WASM Draw Owns Mesh And Validates Texture Shaders

**Files:**

- Modify: `rust/crates/belfast-wasm/src/draw.rs`
- Modify: `rust/crates/belfast-wasm/src/device.rs`
- Modify: `rust/crates/belfast-wasm/src/resources.rs`
- Modify: `rust/web/examples/src/examples/colored-triangle.ts`

**Interfaces:**

- Consumes: core `Draw::validate_for_render`, `Mesh::layout_signature`, and current Naga shader validation.
- Produces: shared `DrawState`, `ShaderResourceLayout`, `Draw.setMesh(mesh)`, `Draw::render`, and mesh-free `Device.render(draw)`.

- [ ] **Step 1: Add failing shader-resource and mesh-replacement tests**

Add host-testable cases in `draw.rs`:

```rust
#[test]
fn accepts_supported_texture_and_sampler_pair() {
    let layout = validate_draw_interface_with_limit(TEXTURE_SHADER, TRIANGLE_ATTRIBUTES, 16)
        .unwrap();
    assert_eq!(
        layout,
        ShaderResourceLayout::TextureSampler {
            group: 0,
            texture_binding: 0,
            sampler_binding: 1,
        }
    );
}

#[test]
fn rejects_storage_texture_and_comparison_sampler() {
    assert!(validate_draw_interface_with_limit(STORAGE_TEXTURE_SHADER, &[], 16)
        .unwrap_err()
        .contains("unsupported shader resource"));
    assert!(validate_draw_interface_with_limit(COMPARISON_SAMPLER_SHADER, &[], 16)
        .unwrap_err()
        .contains("filtering sampler"));
}

#[test]
fn rejects_incomplete_texture_sampler_pair() {
    assert_eq!(
        validate_draw_interface_with_limit(TEXTURE_ONLY_SHADER, &[], 16),
        Err("sampled texture requires one filtering sampler in the same bind group".into())
    );
}
```

Add a pure helper test proving a replacement layout must equal the pipeline
layout signature.

- [ ] **Step 2: Run the draw tests and confirm RED**

Run:

```bash
cd rust
cargo test -p belfast-wasm draw::tests
```

Expected: failures because every global binding is still rejected and the
validator returns only `()`.

- [ ] **Step 3: Implement owned draw state and exact shader resource layout**

Introduce these internal types in `draw.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ShaderResourceLayout {
    None,
    TextureSampler {
        group: u32,
        texture_binding: u32,
        sampler_binding: u32,
    },
}

pub(crate) struct DrawState {
    draw: belfast::Draw,
    mesh: RefCell<belfast::Mesh>,
    resources: ShaderResourceLayout,
}

#[wasm_bindgen(js_name = Draw)]
pub struct WasmDraw {
    pub(crate) state: Rc<DrawState>,
}
```

Change the constructor to consume `WasmMesh`; add
`WasmMesh::into_inner(self) -> belfast::Mesh`. Add:

```rust
#[wasm_bindgen(js_name = setMesh)]
pub fn set_mesh(&self, mesh: WasmMesh) -> Result<(), JsValue>;
```

Validate creator device and identical `MeshLayoutSignature` before replacement.
The validator must inspect Naga globals and accept exactly one non-arrayed,
non-multisampled `texture_2d<f32>` and one non-comparison sampler in group zero.
Reject every other bound resource synchronously and return the discovered
`ShaderResourceLayout` from shader validation.

Update `Device.render` to accept only `&WasmDraw` and draw the mesh stored in
`DrawState`. Update the colored triangle TypeScript constructor/call and remove
the separate `mesh.free()` cleanup.

- [ ] **Step 4: Run host, wasm32, and TypeScript checks**

Run:

```bash
cd rust
cargo test -p belfast-wasm
cargo check -p belfast-wasm --target wasm32-unknown-unknown
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
cd web/examples
pnpm typecheck
```

Expected: all commands pass and generated declarations show
`render(draw: Draw): void` and `setMesh(mesh: Mesh): void`.

- [ ] **Step 5: Commit owned Draw behavior**

```bash
git add rust/crates/belfast-wasm/src rust/web/examples/src/examples/colored-triangle.ts
git commit -m "feat(rust)!: make wasm draw own its mesh"
```

---

### Task 3: WASM Texture And Typed Bind Groups

**Files:**

- Create: `rust/crates/belfast-wasm/src/texture.rs`
- Create: `rust/crates/belfast-wasm/src/bind_group.rs`
- Modify: `rust/crates/belfast-wasm/src/lib.rs`
- Modify: `rust/crates/belfast-wasm/Cargo.toml`

**Interfaces:**

- Consumes: `Texture::create_2d`, `DrawState`, and `ShaderResourceLayout`.
- Produces: `Texture.fromImageBitmap`, `BindGroup.fromTexture`, `TextureState`, and `BindGroupState` used by frame commands.

- [ ] **Step 1: Add failing option and compatibility tests**

Add pure tests for these exact parsers and validators:

```rust
#[test]
fn texture_options_default_to_external_image_settings() {
    let options = TextureOptionsInput::default().resolve().unwrap();
    assert_eq!(options.format, wgpu::TextureFormat::Rgba8UnormSrgb);
    assert_eq!(options.mag_filter, wgpu::FilterMode::Linear);
    assert!(options.flip_y);
}

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
```

Also assert unknown `format`, `magFilter`, and `minFilter` strings are rejected.

- [ ] **Step 2: Run the new unit tests and confirm RED**

Run:

```bash
cd rust
cargo test -p belfast-wasm texture::tests
cargo test -p belfast-wasm bind_group::tests
```

Expected: modules and helper types are undefined.

- [ ] **Step 3: Implement `Texture.fromImageBitmap`**

Enable `web-sys` feature `ImageBitmap`. Define a serde descriptor with
`deny_unknown_fields` and fields `label`, `flipY`, `format`, `magFilter`, and
`minFilter`. Support only:

- formats: `rgba8unorm`, `rgba8unorm-srgb`;
- filters: `nearest`, `linear`.

On wasm32, create the core texture with
`TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT`, then call:

```rust
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
        width: bitmap.width(),
        height: bitmap.height(),
        depth_or_array_layers: 1,
    },
);
```

Store the core texture in `Rc<TextureState>` and expose `width` and `height`
getters.

- [ ] **Step 4: Implement `BindGroup.fromTexture`**

Store:

```rust
pub(crate) struct BindGroupState {
    bind_group: belfast::BindGroup,
    draw: Rc<DrawState>,
    source: BindGroupSource,
}

enum BindGroupSource {
    Texture(Rc<TextureState>),
}
```

Parse `{ groupIndex, textureBinding, samplerBinding, label }`, reject unknown
fields, compare values with `DrawState.resources`, validate texture/draw/device
identity, fetch `draw.get_bind_group_layout(groupIndex)`, and create entries for
the texture view and sampler. Retaining `BindGroupSource` keeps sampled resources
alive.

- [ ] **Step 5: Run the complete facade verification**

Run:

```bash
cd rust
cargo fmt --all -- --check
cargo test -p belfast-wasm
cargo clippy -p belfast-wasm --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
```

Expected: all commands pass.

- [ ] **Step 6: Commit texture and bind-group facades**

```bash
git add rust/crates/belfast-wasm
git commit -m "feat(rust): expose wasm sampled textures"
```

---

### Task 4: Render Targets And Multi-Pass Frame Commands

**Files:**

- Create: `rust/crates/belfast-wasm/src/render_target.rs`
- Create: `rust/crates/belfast-wasm/src/frame.rs`
- Modify: `rust/crates/belfast-wasm/src/bind_group.rs`
- Modify: `rust/crates/belfast-wasm/src/device.rs`
- Modify: `rust/crates/belfast-wasm/src/lib.rs`

**Interfaces:**

- Consumes: `DrawState`, `BindGroupState`, core `RenderTarget`, and existing surface recovery/error state.
- Produces: `RenderTarget.create`, `RenderTarget.resize`, `BindGroup.fromRenderTarget`, `Device.beginFrame`, `Frame.bindTarget`, `Frame.render`, and `Frame.submit`.

- [ ] **Step 1: Add failing logical-pass planner tests**

Keep the state machine host-testable without a browser surface. Test this exact
sequence:

```rust
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
    assert_eq!(FramePlan::new().finish(), Err("frame contains no draw commands".into()));
}
```

Add validation tests for missing required bind groups, unexpected bind groups,
wrong draw identity, wrong device, and mismatched render-target format.

- [ ] **Step 2: Run frame tests and confirm RED**

Run:

```bash
cd rust
cargo test -p belfast-wasm frame::tests
```

Expected: `FramePlan` and frame validation do not exist.

- [ ] **Step 3: Implement the render-target facade**

Store `Rc<RefCell<belfast::RenderTarget>>` plus creator device. Parse
`{ width, height, label }` with unknown-field rejection. Use the device surface
format, sample count one, and no depth. Expose `width`, `height`, and:

```rust
#[wasm_bindgen(js_name = resize)]
pub fn resize(&self, width: u32, height: u32) -> Result<bool, JsValue>;
```

Return `true` only when dimensions changed. Validate positive dimensions and the
device's `max_texture_dimension_2d` before calling the core resize method.

Extend `BindGroupSource` with the render-target shared state and implement
`BindGroup.fromRenderTarget` with the same binding options and draw/device
checks as `fromTexture`.

- [ ] **Step 4: Implement frame recording and submission**

Represent commands as owned shared handles:

```rust
enum FrameTarget {
    Canvas,
    RenderTarget(Rc<RefCell<belfast::RenderTarget>>),
}

struct RenderCommand {
    draw: Rc<DrawState>,
    bind_group: Option<Rc<BindGroupState>>,
}

struct LogicalPass {
    target: FrameTarget,
    clear_color: wgpu::Color,
    commands: Vec<RenderCommand>,
}
```

`bindTarget(Option<&WasmRenderTarget>, JsValue)` closes a non-empty current pass
and starts another. `render(&WasmDraw, Option<&WasmBindGroup>)` defaults to a
canvas pass and validates resource requirements immediately. `submit(self)`
creates one encoder, emits each logical pass, submits once, and presents the
owned surface texture. Parse pass options through a
`#[serde(deny_unknown_fields)]` descriptor and reject non-finite clear-color
components before command recording.

Before opening each wgpu render pass, collect and retain the `Ref<Mesh>` guards
for every command in that logical pass. Keep those guards, the target
`Ref<RenderTarget>`, draw `Rc`s, and bind-group `Rc`s alive until the pass ends;
do not borrow a mesh temporarily inside the draw loop because wgpu ties bound
buffer lifetimes to the render pass.

Move surface acquisition from `Device.render` into `Device.beginFrame` while
preserving timeout/occlusion skipping, outdated reconfiguration, lost-surface
recreation, device-error propagation, and suboptimal reconfiguration on the
next frame. Make `Device.render(draw)` call begin, render, and submit.

- [ ] **Step 5: Run all Rust and wasm verification**

Run:

```bash
cd rust
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
```

Inspect generated declarations for:

```ts
beginFrame(): Frame | undefined;
bindTarget(target?: RenderTarget | null, options?: unknown): void;
render(draw: Draw, bindGroup?: BindGroup | null): void;
submit(): void;
```

- [ ] **Step 6: Commit frame and render-target APIs**

```bash
git add rust/crates/belfast-wasm
git commit -m "feat(rust): add wasm multi-pass frames"
```

---

### Task 5: Browser Texture Example And Optimized Asset

**Files:**

- Create: `rust/web/examples/public/scattered003.jpg`
- Create: `rust/web/examples/src/examples/texture.ts`
- Create: `rust/web/examples/src/shaders/texture.wgsl`
- Modify: `rust/web/examples/src/main.ts`

**Interfaces:**

- Consumes: `Texture.fromImageBitmap`, `BindGroup.fromTexture`, owned `Draw`, `Draw.setMesh`, and `Frame`.
- Produces: browser route `?example=texture`.

- [ ] **Step 1: Convert and verify the image asset**

Run:

```bash
mkdir -p rust/web/examples/public
sips -Z 1980 -s format jpeg -s formatOptions 88 /Users/yi-wenlin/Desktop/scattered003.png --out rust/web/examples/public/scattered003.jpg
sips -g pixelWidth -g pixelHeight -g format rust/web/examples/public/scattered003.jpg
```

Expected: JPEG, 1400 x 1980. Confirm the file is materially smaller than 32 MB
with `ls -lh`.

- [ ] **Step 2: Add the textured quad shader**

Create `texture.wgsl` with position location 0, UV location 1, and the supported
texture/sampler pair:

```wgsl
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var image_texture: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

@vertex
fn vs_main(@location(0) position: vec2f, @location(1) uv: vec2f) -> VertexOutput {
    return VertexOutput(vec4f(position, 0.0, 1.0), uv);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return textureSample(image_texture, image_sampler, input.uv);
}
```

- [ ] **Step 3: Implement JS loading and aspect-fit geometry**

Fetch `/scattered003.jpg`, reject non-OK responses, create an `ImageBitmap`, pass
it to `Texture.fromImageBitmap`, and close it in `finally`.

Use this scale rule:

```ts
const imageAspect = imageWidth / imageHeight;
const canvasAspect = canvas.width / canvas.height;
const scaleX = imageAspect > canvasAspect ? 1 : imageAspect / canvasAspect;
const scaleY = imageAspect > canvasAspect ? canvasAspect / imageAspect : 1;
```

Build six `float32x2` positions and matching UVs. Construct `Draw` once, create
the texture bind group, and on a canvas dimension change create a compatible
replacement mesh for `draw.setMesh(mesh)`. Render with:

```ts
const frame = device.beginFrame();
if (frame) {
  frame.bindTarget(null);
  frame.render(draw, bindGroup);
  frame.submit();
}
```

Free replaced JS buffer wrappers after their data has been cloned into the mesh;
cleanup stops animation and frees bind group, draw, texture, and device.

- [ ] **Step 4: Register the example and run frontend checks**

Add `texture` to the `examples` map in `main.ts` and run:

```bash
cd rust/web/examples
pnpm typecheck
pnpm build
```

Expected: both commands pass and the production output contains the JPEG and
Wasm asset.

- [ ] **Step 5: Commit the texture example**

```bash
git add rust/web/examples/public/scattered003.jpg rust/web/examples/src
git commit -m "feat(rust): add wasm texture example"
```

---

### Task 6: Browser Render-To-Texture Example And Final Verification

**Files:**

- Create: `rust/web/examples/src/examples/render-to-texture.ts`
- Create: `rust/web/examples/src/shaders/render-target-present.wgsl`
- Modify: `rust/web/examples/src/main.ts`
- Modify: `rust/README.md`
- Modify: `rust/docs/rust-wgpu-api-parity.md`

**Interfaces:**

- Consumes: `RenderTarget`, `BindGroup.fromRenderTarget`, frame target switching, and the colored triangle data/shader.
- Produces: browser route `?example=render-to-texture`, documentation, and final evidence.

- [ ] **Step 1: Add the split presentation shader**

Use a fullscreen triangle derived from `@builtin(vertex_index)` and invert Y for
the texture coordinates. The fragment stage must preserve the left side and use
perceptual luminance on the right:

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let color = textureSample(source_texture, source_sampler, input.uv);
    let luminance = dot(color.rgb, vec3f(0.2126, 0.7152, 0.0722));
    let output_rgb = select(vec3f(luminance), color.rgb, input.uv.x < 0.5);
    return vec4f(output_rgb, color.a);
}
```

- [ ] **Step 2: Implement the two logical passes**

Create a source `Draw` from the colored triangle mesh, a presentation `Draw`
from `new Mesh(3)`, and a canvas-sized `RenderTarget`. Create the presentation
bind group from that target. Record:

```ts
const frame = device.beginFrame();
if (frame) {
  frame.bindTarget(target, { clearColor: { r: 0.02, g: 0.025, b: 0.04, a: 1 } });
  frame.render(source);
  frame.bindTarget(null, { clearColor: { r: 0, g: 0, b: 0, a: 1 } });
  frame.render(present, presentBindGroup);
  frame.submit();
}
```

When canvas dimensions change, resize the target, free the old presentation bind
group, and recreate it before recording the frame. Cleanup frees both draws,
both bind groups where applicable, the target, source buffers, and device.

- [ ] **Step 3: Register and document both examples**

Register `render-to-texture` in `main.ts`. Add these URLs and the existing build
commands to `rust/README.md`:

```text
http://127.0.0.1:5173/?example=texture
http://127.0.0.1:5173/?example=render-to-texture
```

Mark WASM `Texture`, `BindGroup`, `RenderTarget`, and multi-pass rendering as
`Partial` in the parity table, listing the exact unsupported resource classes
from the spec.

- [ ] **Step 4: Run the full current-HEAD verification matrix**

Run:

```bash
cd rust
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
cd web/examples
pnpm typecheck
pnpm build
```

Expected: every command exits zero.

- [ ] **Step 5: Verify both examples in a WebGPU browser**

Start Vite on an available localhost port. For `?example=texture`, verify the
image is centered, completely visible, aspect-correct, nonblank, and remains so
after desktop/mobile resize. For `?example=render-to-texture`, verify the
triangle's left-side samples retain distinct RGB channels while corresponding
right-side samples have equal RGB channels. Confirm there are no page, console,
Wasm, or WebGPU errors.

Capture screenshots and record canvas CSS/backing dimensions and representative
pixel samples in the SDD evidence directory.

- [ ] **Step 6: Request final review and fix blocking findings**

Review the complete range from the plan commit through current HEAD against the
spec. Fix every Critical or Important finding, rerun affected tests, then rerun
the full verification matrix before completion.

- [ ] **Step 7: Commit the render-to-texture example and docs**

```bash
git add rust/web/examples/src rust/README.md rust/docs/rust-wgpu-api-parity.md
git commit -m "feat(rust): add wasm render target example"
```
