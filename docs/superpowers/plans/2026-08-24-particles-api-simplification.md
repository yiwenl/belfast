# Particles API Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the branch-only WebAssembly `DepthDraw` and `ShadowMap` façades, reuse `Draw` and `Texture`, preserve planned shadow depth loads, and remove the unnecessary camera-helper device argument.

**Architecture:** Visible and depth-only pipelines share `WasmDraw` and its existing resource-layout validation. Shadow targets and shadow bind groups share `WasmTexture`/`TextureState`; the frame retains only the target-specific scheduling branch needed to encode a depth attachment. Depth load behavior is explicit at the native `Texture::begin_depth_pass` boundary.

**Tech Stack:** Rust 2021, wgpu 30, wasm-bindgen, serde-wasm-bindgen, naga WGSL validation, TypeScript, Vite, pnpm.

**Spec:** `docs/superpowers/specs/2026-08-24-particles-api-simplification-design.md`

## Global Constraints

- Backward compatibility with the branch-only `DepthDraw` and `ShadowMap` exports is intentionally not preserved.
- Add no compatibility wrappers, deprecated aliases, generic render-resource traits, or texture-kind hierarchy.
- `depthOnly` is an immediately used `DrawOptions` flag and is mutually exclusive with `depth`.
- A resource-free depth-only shader must render with zero bind groups; a group-0 uniform depth shader must require its matching bind group.
- Only `Depth32Float` textures created by `Texture.createShadowMap` may be shadow targets or shadow-map bind-group sources.
- `Texture::begin_depth_pass` takes an explicit `wgpu::LoadOp<f32>`; no options struct or overload is added.
- `CameraHelper::set_matrix` and WASM `CameraHelper.setMatrix` use the internally owned device.

## File Map

- `crates/belfast-wasm/src/draw.rs`: parse `depthOnly`, select depth shader validation and native depth-only pipeline construction.
- `crates/belfast-wasm/src/frame.rs`: keep one draw path, resolve registered shadow textures, and forward planned depth loads.
- `crates/belfast-wasm/src/bind_group.rs`: keep one draw owner and retain shadow resources through `TextureState`.
- `crates/belfast-wasm/src/texture.rs`: own image and shadow-map creation in one WASM texture façade.
- `crates/belfast/src/texture.rs`: require an explicit depth load operation.
- `crates/belfast/src/helpers/camera_helper.rs`: update positions with its owned device.
- `crates/belfast-wasm/src/camera_helper.rs`: expose the device-free matrix update.
- `crates/belfast/examples/particles.rs`, `web/particles/src/main.ts`: exercise the simplified native and WASM APIs.
- `crates/belfast-wasm/src/depth_draw.rs`, `crates/belfast-wasm/src/shadow_map.rs`: delete after their consumers migrate.
- `crates/belfast-wasm/src/lib.rs`, `docs/rust-wgpu-api-parity.md`: remove obsolete exports and document the unified surface.

---

### Task 1: Fold depth-only rendering into `Draw`

**Files:**

- Modify: `crates/belfast-wasm/src/draw.rs`
- Modify: `crates/belfast-wasm/src/bind_group.rs`
- Modify: `crates/belfast-wasm/src/frame.rs`
- Modify: `crates/belfast-wasm/src/lib.rs`
- Modify: `crates/belfast-wasm/src/device.rs`
- Modify: `web/particles/src/main.ts`
- Delete: `crates/belfast-wasm/src/depth_draw.rs`
- Test: unit tests in `crates/belfast-wasm/src/draw.rs` and `crates/belfast-wasm/src/frame.rs`

**Interfaces:**

- Consumes: `belfast::DrawOptions::depth_only(label)`, `ShaderResourceLayout::bind_group_count()`.
- Produces: `DrawOptions.depthOnly?: boolean`; all draw submission and bind-group ownership use `WasmDraw`/`DrawState`.

- [ ] **Step 1: Write failing tests for the desired `Draw` API**

Add a small pure selector in the test's wished-for API and test both the depth shader path and conflicting flags:

```rust
const RESOURCE_FREE_DEPTH_SHADER: &str = r#"
@vertex
fn vs_main(@location(0) position: vec3f) -> @builtin(position) vec4f {
    return vec4f(position, 1.0);
}

@fragment
fn fs_main() {}
"#;

#[test]
fn depth_only_draw_accepts_a_resource_free_shader() {
    let resources = parse_and_validate_pipeline_shader(
        RESOURCE_FREE_DEPTH_SHADER,
        &[(0, wgpu::VertexFormat::Float32x3)],
        16,
        true,
    )
    .unwrap();
    assert_eq!(resources, ShaderResourceLayout::None);
}

#[test]
fn depth_only_and_canvas_depth_are_mutually_exclusive() {
    assert_eq!(
        validate_depth_options(true, true),
        Err("depth and depthOnly cannot both be true".into())
    );
}
```

Add this frame regression assertion beside the existing render validation tests:

```rust
#[test]
fn resource_free_draw_accepts_zero_bind_groups() {
    assert!(validate_render_command(DeviceKey(1), draw(7, 1, 0), &[]).is_ok());
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test -p belfast-wasm depth_only_draw_accepts_a_resource_free_shader --lib
```

Expected: compilation fails because `parse_and_validate_pipeline_shader` and `validate_depth_options` do not exist.

- [ ] **Step 3: Implement only the new `Draw` mode**

Extend the custom TypeScript interface and deserializer:

```rust
export interface DrawOptions {
  label?: string;
  primitive?: DrawPrimitiveState;
  depth?: boolean;
  depthOnly?: boolean;
}

#[serde(default)]
depth_only: bool,
```

Add pure helpers without introducing a mode enum:

```rust
fn validate_depth_options(depth: bool, depth_only: bool) -> Result<(), String> {
    if depth && depth_only {
        Err("depth and depthOnly cannot both be true".into())
    } else {
        Ok(())
    }
}

fn parse_and_validate_pipeline_shader(
    shader_code: &str,
    mesh_attributes: &[(u32, wgpu::VertexFormat)],
    max_inter_stage_shader_variables: u32,
    depth_only: bool,
) -> Result<ShaderResourceLayout, String> {
    if depth_only {
        parse_and_validate_depth_shader(
            shader_code,
            mesh_attributes,
            max_inter_stage_shader_variables,
        )
    } else {
        parse_and_validate_draw_shader(
            shader_code,
            mesh_attributes,
            max_inter_stage_shader_variables,
        )
    }
}
```

In `WasmDraw::new`, validate the flags, call the selector, and construct either `belfast::DrawOptions::depth_only(&label)` or the existing color `DrawOptions`. Apply primitive overrides to both paths; apply the existing fixed canvas depth state only to the color path.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
cargo test -p belfast-wasm depth_only --lib
cargo test -p belfast-wasm resource_free_draw_accepts_zero_bind_groups --lib
```

Expected: all selected tests pass.

- [ ] **Step 5: Migrate depth consumers and remove duplicate state**

Make the mechanical substitution:

```ts
const shadowDraw = new Draw(device, shadowShaderCode, mesh, {
  label: "ParticleShadow",
  depthOnly: true,
});
```

Then remove `DepthDrawState`, `WasmDepthDraw`, `DepthDrawKey`, `DepthDrawValidation`, `BindGroupOwner::DepthDraw`, `Drawable::DepthDraw`, `PlannedDrawable::DepthDraw`, `RenderCommand::DepthDraw`, `WasmFrame.depth_draws`, `render_depth_draw`, `from_depth_draw_buffer`, the module/export, and the source file. Route the shadow draw through the existing `Draw` branches and keep `instanceCount` behavior unchanged.

- [ ] **Step 6: Verify the refactor stays GREEN**

Run:

```bash
cargo test -p belfast-wasm --lib
cargo check -p belfast-wasm --target wasm32-unknown-unknown
pnpm typecheck
rg -n "DepthDraw|DepthDrawState|DepthDrawKey" crates/belfast-wasm web/particles
```

Expected: tests/check/typecheck pass; `rg` returns no matches.

- [ ] **Step 7: Commit the depth-only unification**

```bash
git add crates/belfast-wasm/src/draw.rs crates/belfast-wasm/src/bind_group.rs crates/belfast-wasm/src/frame.rs crates/belfast-wasm/src/lib.rs crates/belfast-wasm/src/device.rs web/particles/src/main.ts
git add -u crates/belfast-wasm/src/depth_draw.rs
git commit -m "refactor(wasm)!: fold DepthDraw into Draw" -m "BREAKING CHANGE: construct depth-only pipelines with Draw and the depthOnly option."
```

---

### Task 2: Fold shadow maps into `Texture` and preserve depth loads

**Files:**

- Modify: `crates/belfast/src/texture.rs`
- Modify: `crates/belfast/tests/texture.rs`
- Modify: `crates/belfast/examples/particles.rs`
- Modify: `crates/belfast-wasm/src/texture.rs`
- Modify: `crates/belfast-wasm/src/bind_group.rs`
- Modify: `crates/belfast-wasm/src/frame.rs`
- Modify: `crates/belfast-wasm/src/lib.rs`
- Modify: `web/particles/src/main.ts`
- Delete: `crates/belfast-wasm/src/shadow_map.rs`
- Test: unit tests in `crates/belfast-wasm/src/texture.rs` and `crates/belfast-wasm/src/frame.rs`; integration test in `crates/belfast/tests/texture.rs`

**Interfaces:**

- Consumes: `belfast::TextureOptions::shadow_map(label)`, `TextureState`, `PlannedLoadOp`.
- Produces: `Texture.createShadowMap(device, { size?, label? }): Texture`; `Texture::begin_depth_pass(encoder, load_op)`.

- [ ] **Step 1: Write failing tests for explicit load and shadow texture creation**

Add an integration-test call using the desired native signature:

```rust
#[test]
fn depth_pass_accepts_an_explicit_load_operation() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let texture = Texture::create_2d(
        &device,
        16,
        16,
        TextureOptions::shadow_map("DepthLoadTest"),
    )
    .unwrap();
    let mut encoder = device
        .gpu()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    drop(texture.begin_depth_pass(&mut encoder, wgpu::LoadOp::Load));
}
```

Move the shadow input test to `texture.rs` and assert the wished-for resolver:

```rust
#[test]
fn shadow_map_options_resolve_to_a_depth_texture() {
    let (size, options) = resolve_shadow_map_options(ShadowMapOptionsInput::default());
    assert_eq!(size, 1024);
    assert_eq!(options.format, wgpu::TextureFormat::Depth32Float);
    assert_eq!(options.compare, Some(wgpu::CompareFunction::Less));
}
```

Extend the existing frame split test to assert that the second shadow pass has `PlannedLoadOp::Load`:

```rust
assert!(matches!(
    &ops[2],
    PlannedOp::RenderPass(PlannedRenderPass {
        target: TargetKey::Shadow(_),
        load_op: PlannedLoadOp::Load,
        ..
    })
));
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test -p belfast depth_pass_accepts_an_explicit_load_operation --test texture
cargo test -p belfast-wasm shadow_map_options_resolve_to_a_depth_texture --lib
```

Expected: the first fails to compile because `begin_depth_pass` accepts one argument after `self`; the second fails because the shadow resolver is not in `texture.rs`.

- [ ] **Step 3: Make native depth loading explicit**

Change the native method to:

```rust
pub fn begin_depth_pass<'a>(
    &'a self,
    encoder: &'a mut wgpu::CommandEncoder,
    load_op: wgpu::LoadOp<f32>,
) -> wgpu::RenderPass<'a>
```

Use `load_op` in the depth attachment operations. Update native particles to pass `wgpu::LoadOp::Clear(1.0)`.

- [ ] **Step 4: Add shadow creation to the existing WASM texture façade**

Move `ShadowMapOptionsInput` into `texture.rs` and add:

```rust
fn resolve_shadow_map_options(
    input: ShadowMapOptionsInput,
) -> (u32, belfast::TextureOptions) {
    let size = input.size.unwrap_or(1024).max(1);
    let options = belfast::TextureOptions::shadow_map(
        input.label.unwrap_or_else(|| "ShadowMap".into()),
    );
    (size, options)
}
```

Expose `Texture.createShadowMap`, returning a registered `WasmTexture` backed by `Rc<TextureState>`. Add `TextureState::is_shadow_map()` using `texture.format() == wgpu::TextureFormat::Depth32Float` and add `__frameHandle` to clone the existing state. Do not add a second state struct.

- [ ] **Step 5: Migrate bind groups and frame targets to `TextureState`**

Change `BindGroup.fromShadowMap` to take `&WasmTexture`, reject `!texture.state.is_shadow_map()`, and retain `BindGroupSource::Texture(texture.state.clone())`; remove the `ShadowMap` source variant.

In `frame.rs`, register and parse shadow textures, store `Rc<TextureState>` in the shadow-target map, and materialize the existing shadow target branch from that state. The encoder call must forward the planned value:

```rust
FrameTarget::ShadowMap(shadow_map) => {
    let mut pass = shadow_map
        .texture()
        .begin_depth_pass(encoder, depth_load);
    encode_render_commands(&mut pass, &commands, &draw_meshes);
}
```

Update web particles:

```ts
const shadowMap = Texture.createShadowMap(device, {
  size: SHADOW_MAP_SIZE,
  label: "ParticleShadowMap",
});
```

Remove `ShadowMapState`, `WasmShadowMap`, its module/export, and `shadow_map.rs`.

- [ ] **Step 6: Run focused and cross-target verification**

Run:

```bash
cargo test -p belfast depth_pass_accepts_an_explicit_load_operation --test texture
cargo test -p belfast-wasm shadow --lib
cargo check -p belfast-wasm --target wasm32-unknown-unknown
cargo check -p belfast --examples --target wasm32-unknown-unknown
pnpm typecheck
rg -n "WasmShadowMap|ShadowMapState|from crate::shadow_map" crates/belfast-wasm
```

Expected: tests/check/typecheck pass; `rg` returns no matches.

- [ ] **Step 7: Commit the texture unification**

```bash
git add crates/belfast/src/texture.rs crates/belfast/tests/texture.rs crates/belfast/examples/particles.rs crates/belfast-wasm/src/texture.rs crates/belfast-wasm/src/bind_group.rs crates/belfast-wasm/src/frame.rs crates/belfast-wasm/src/lib.rs web/particles/src/main.ts
git add -u crates/belfast-wasm/src/shadow_map.rs
git commit -m "refactor(wasm)!: fold ShadowMap into Texture" -m "BREAKING CHANGE: create shadow targets with Texture.createShadowMap."
```

---

### Task 3: Remove the camera-helper device parameter

**Files:**

- Modify: `crates/belfast/src/helpers/camera_helper.rs`
- Modify: `crates/belfast/tests/camera_helper.rs`
- Modify: `crates/belfast-wasm/src/camera_helper.rs`

**Interfaces:**

- Consumes: `Buffer::device()` and the existing `CameraHelper.positions` buffer.
- Produces: `CameraHelper::set_matrix(&self, view_proj)` and WASM `CameraHelper.setMatrix(viewProj)`.

- [ ] **Step 1: Write the compile-failing desired API test**

Extend `camera_helper_builds_with_a_compatible_camera_layout` after unwrapping the helper:

```rust
let helper = helper.unwrap();
let updated = OrthographicCamera::new(-2.0, 2.0, -2.0, 2.0, 1.0, 4.0);
helper
    .set_matrix(&updated.view_projection_matrix())
    .expect("update camera helper matrix");
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test -p belfast camera_helper_builds_with_a_compatible_camera_layout --test camera_helper
```

Expected: compilation fails because `set_matrix` still requires a `&Device` argument.

- [ ] **Step 3: Remove the external device argument**

Implement:

```rust
pub fn set_matrix(&self, view_proj: &[f32; 16]) -> BelfastResult<()> {
    let positions = frustum_positions(view_proj)?;
    self.positions
        .write(self.positions.device(), &positions, 0);
    Ok(())
}
```

Change WASM `set_matrix` to accept only `view_proj` and call `self.state.helper.set_matrix(&matrix)`.

- [ ] **Step 4: Verify GREEN across native and WASM**

Run:

```bash
cargo test -p belfast camera_helper --tests
cargo check -p belfast-wasm --target wasm32-unknown-unknown
rg -n "set_matrix\(&device|setMatrix\(device" crates web
```

Expected: tests/check pass; `rg` returns no matches.

- [ ] **Step 5: Commit the API cleanup**

```bash
git add crates/belfast/src/helpers/camera_helper.rs crates/belfast/tests/camera_helper.rs crates/belfast-wasm/src/camera_helper.rs
git commit -m "refactor(camera): own matrix update device"
```

---

### Task 4: Synchronize documentation and run the full gate

**Files:**

- Modify: `docs/rust-wgpu-api-parity.md`
- Modify: `crates/belfast/examples/README.md`
- Verify: all files changed since `4d16fb4`

**Interfaces:**

- Consumes: the final public API from Tasks 1–3.
- Produces: documentation that names only `Draw`/`Texture` and accurately states one million particles.

- [ ] **Step 1: Update API parity and example documentation**

Replace the separate WASM façade claims with the unified API:

```markdown
| `ShadowMap` | Done | Done | Done | Native and WASM use `Texture` shadow-map factories and depth-pass helpers. |
| `DepthDraw` | Done | Done | Done | Native and WASM use `Draw` depth-only construction. |
```

Change the examples table from `10k instanced billboard discs` to `1M instanced billboard discs with compute simulation and shadows`.

- [ ] **Step 2: Format only changed text/code files**

Run:

```bash
pnpm exec prettier --write web/particles/src/main.ts docs/rust-wgpu-api-parity.md crates/belfast/examples/README.md docs/superpowers/plans/2026-08-24-particles-api-simplification.md
cargo fmt --all
```

- [ ] **Step 3: Run the full verification gate**

Run each command and require exit code 0:

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
cargo check -p belfast --examples --target wasm32-unknown-unknown
pnpm typecheck
pnpm --filter @belfast/web-examples build
git diff --check 4d16fb4...HEAD
```

Also run:

```bash
rg -n "DepthDraw|WasmDepthDraw|DepthDrawState|WasmShadowMap|ShadowMapState" crates/belfast-wasm web docs/rust-wgpu-api-parity.md
```

Expected: no obsolete implementation or public export remains. Mentions in the parity table are allowed only as legacy component names whose implementation is explicitly `Draw`/`Texture`.

- [ ] **Step 4: Inspect the final diff for scope and dead code**

Run:

```bash
git diff --stat 4d16fb4...HEAD
git diff 4d16fb4...HEAD -- crates/belfast-wasm/src crates/belfast/src web/particles docs/rust-wgpu-api-parity.md crates/belfast/examples/README.md
```

Confirm there is no compatibility alias, unused option, unreferenced helper, new one-field wrapper, or unrelated file edit.

- [ ] **Step 5: Commit documentation if it was not included earlier**

```bash
git add docs/rust-wgpu-api-parity.md crates/belfast/examples/README.md
git commit -m "docs: sync unified particle APIs"
```
