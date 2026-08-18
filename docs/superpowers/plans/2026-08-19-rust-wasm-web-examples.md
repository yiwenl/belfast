# Rust WASM Web Examples Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Vite-based Rust WASM example gallery whose first example renders a colored triangle from JavaScript-provided positions, colors, and WGSL.

**Architecture:** Keep browser-only surface ownership and JavaScript conversion in `belfast-wasm`, while delegating buffers, meshes, pipelines, and drawing to the reusable `belfast` crate. The WebAssembly facade exposes Belfast-named classes and records complete render passes in Rust because `wgpu` encoder and pass lifetimes cannot safely cross into JavaScript.

**Tech Stack:** Rust 2021, `wgpu` 0.20, `wasm-bindgen`, `serde-wasm-bindgen`, `web-sys`, `wasm-pack`, TypeScript 5.7, Vite 6, pnpm 9.

**Spec:** `docs/superpowers/specs/2026-08-19-rust-wasm-web-examples-design.md`

## Global Constraints

- Native examples remain in `rust/crates/belfast/examples`.
- Browser-specific types and camelCase conversion remain in `rust/crates/belfast-wasm`; `rust/crates/belfast` must not depend on `wasm-bindgen` or `web-sys`.
- Browser examples live in the single Vite project `rust/web/examples`.
- Generated `wasm-pack` output remains under ignored `rust/pkg/belfast-wasm`.
- Positions and colors are separate JavaScript `Float32Array` values using `float32x2` and `float32x3` layouts.
- JavaScript supplies WGSL source; no colored-triangle-specific renderer is added to production Rust code.
- Raw `wgpu` encoders, render passes, surfaces, and WebGPU handles do not cross the WebAssembly boundary.
- All user-provided JavaScript data returns `Result` errors instead of panicking.

---

## File Structure

- `rust/crates/belfast-wasm/src/lib.rs`: module wiring plus the existing camera and uniform bindings.
- `rust/crates/belfast-wasm/src/bindings.rs`: serde input descriptors, string-to-`wgpu` conversion, and validation tests.
- `rust/crates/belfast-wasm/src/resources.rs`: JavaScript-facing `Buffer`, `BufferUsage`, and `Mesh` classes.
- `rust/crates/belfast-wasm/src/draw.rs`: JavaScript-facing `Draw` pipeline class.
- `rust/crates/belfast-wasm/src/device.rs`: headless device compatibility plus browser canvas surface, resize, frame encoding, submission, and presentation.
- `rust/web/examples/src/main.ts`: WebAssembly initialization and example selection.
- `rust/web/examples/src/examples/colored-triangle.ts`: JavaScript data setup and animation loop.
- `rust/web/examples/src/shaders/colored-triangle.wgsl`: editable browser-owned shader.
- `rust/web/examples/src/style.css`: full-viewport canvas layout and visible error state.

### Task 1: Typed Buffer And Mesh Bindings

**Files:**

- Modify: `rust/Cargo.toml`
- Modify: `rust/crates/belfast-wasm/Cargo.toml`
- Modify: `rust/crates/belfast-wasm/src/lib.rs`
- Create: `rust/crates/belfast-wasm/src/bindings.rs`
- Create: `rust/crates/belfast-wasm/src/resources.rs`

**Interfaces:**

- Consumes: `belfast::Buffer::from_data`, `belfast::BufferUsage::vertex`, `belfast::Mesh::add_vertex_buffer`, and `belfast::VertexBufferBinding`.
- Produces: exported JavaScript classes `BufferUsage`, `Buffer`, and `Mesh`; crate-visible `WasmBuffer::inner()` and `WasmMesh::inner()` accessors for later render tasks.

- [ ] **Step 1: Add failing conversion and validation tests**

Create `bindings.rs` tests that establish accepted public values and reject malformed descriptors:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vertex_buffer_usage() {
        assert_eq!(parse_buffer_usage("vertex").unwrap(), belfast::BufferUsage::vertex());
    }

    #[test]
    fn rejects_unknown_buffer_usage() {
        assert_eq!(
            parse_buffer_usage("storage").unwrap_err(),
            "unsupported buffer usage \"storage\""
        );
    }

    #[test]
    fn converts_separate_position_layout() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 8,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "float32x2".into(),
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        };

        let converted = descriptor.try_into_binding().unwrap();
        assert_eq!(converted.array_stride, 8);
        assert_eq!(converted.attributes[0].format, wgpu::VertexFormat::Float32x2);
        assert_eq!(converted.slot, Some(0));
    }

    #[test]
    fn rejects_attribute_past_array_stride() {
        let descriptor = VertexBufferDescriptorInput {
            array_stride: 8,
            attributes: vec![VertexAttributeInput {
                shader_location: 0,
                format: "float32x3".into(),
                offset: 0,
            }],
            slot: Some(0),
            step_mode: None,
        };

        assert_eq!(
            descriptor.try_into_binding().unwrap_err(),
            "vertex attribute at shaderLocation 0 exceeds arrayStride 8"
        );
    }
}
```

- [ ] **Step 2: Run the tests and verify RED**

Run:

```bash
cd rust
cargo test -p belfast-wasm bindings::tests -- --nocapture
```

Expected: compilation fails because `parse_buffer_usage`, `VertexBufferDescriptorInput`, and conversion methods do not exist.

- [ ] **Step 3: Implement descriptor conversion**

Add workspace dependencies `serde = { version = "1", features = ["derive"] }` and `serde-wasm-bindgen = "0.6"`, plus direct `wgpu.workspace = true` in `belfast-wasm`.

Implement these inputs in `bindings.rs`:

```rust
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VertexBufferDescriptorInput {
    pub array_stride: u64,
    pub attributes: Vec<VertexAttributeInput>,
    #[serde(default)]
    pub slot: Option<u32>,
    #[serde(default)]
    pub step_mode: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VertexAttributeInput {
    pub shader_location: u32,
    pub format: String,
    #[serde(default)]
    pub offset: u64,
}

pub(crate) struct ConvertedVertexBinding {
    pub array_stride: u64,
    pub attributes: Vec<belfast::VertexAttributeDescriptor>,
    pub slot: Option<u32>,
    pub step_mode: Option<wgpu::VertexStepMode>,
}
```

Support only `vertex`, `float32x2`, `float32x3`, and step modes `vertex` or `instance` in this slice. Reject zero stride, empty attributes, unknown strings, duplicate shader locations, and any attribute whose `offset + format_size` exceeds `arrayStride`.

- [ ] **Step 4: Run conversion tests and verify GREEN**

Run:

```bash
cd rust
cargo test -p belfast-wasm bindings::tests -- --nocapture
```

Expected: all binding conversion tests pass.

- [ ] **Step 5: Add JavaScript-facing resource classes**

Replace the placeholder `Buffer` and `Mesh` bindings with the following public behavior in `resources.rs`:

```rust
#[wasm_bindgen(js_name = BufferUsage)]
pub struct WasmBufferUsage;

#[wasm_bindgen(js_class = BufferUsage)]
impl WasmBufferUsage {
    #[wasm_bindgen(getter, static_method_of = WasmBufferUsage)]
    pub fn vertex() -> String {
        "vertex".into()
    }
}

#[wasm_bindgen(js_name = Buffer)]
pub struct WasmBuffer {
    inner: belfast::Buffer,
}

#[wasm_bindgen(js_class = Buffer)]
impl WasmBuffer {
    #[wasm_bindgen(js_name = fromData)]
    pub fn from_data(
        device: &WasmDevice,
        values: &[f32],
        usage: &str,
        label: Option<String>,
    ) -> Result<WasmBuffer, JsValue>;

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize;
}

#[wasm_bindgen(js_name = Mesh)]
pub struct WasmMesh {
    inner: belfast::Mesh,
}

#[wasm_bindgen(js_class = Mesh)]
impl WasmMesh {
    #[wasm_bindgen(constructor)]
    pub fn new(vertex_count: u32) -> Result<WasmMesh, JsValue>;

    #[wasm_bindgen(js_name = addVertexBuffer)]
    pub fn add_vertex_buffer(
        self,
        buffer: &WasmBuffer,
        descriptor: JsValue,
    ) -> Result<WasmMesh, JsValue>;
}
```

The owned `self` receiver intentionally lets generated JavaScript preserve Belfast's chaining API. Reject an empty `Float32Array` before creating a GPU buffer. Deserialize `descriptor` with `serde_wasm_bindgen::from_value`, then construct `belfast::VertexBufferBinding` using a clone of the inner Rust buffer.

- [ ] **Step 6: Verify host and WASM compilation**

Run:

```bash
cd rust
cargo test -p belfast-wasm
cargo check -p belfast-wasm --target wasm32-unknown-unknown
```

Expected: both commands pass and generated bindings compile for WebAssembly.

- [ ] **Step 7: Commit typed resources**

```bash
git add rust/Cargo.toml rust/Cargo.lock rust/crates/belfast-wasm/Cargo.toml rust/crates/belfast-wasm/src/lib.rs rust/crates/belfast-wasm/src/bindings.rs rust/crates/belfast-wasm/src/resources.rs
git commit -m "feat(rust): expose wasm mesh resources"
```

### Task 2: Browser Surface And Rust-Owned Render Pass

**Files:**

- Modify: `rust/crates/belfast-wasm/Cargo.toml`
- Modify: `rust/crates/belfast-wasm/src/lib.rs`
- Create: `rust/crates/belfast-wasm/src/device.rs`
- Create: `rust/crates/belfast-wasm/src/draw.rs`

**Interfaces:**

- Consumes: Task 1's `WasmBuffer`, `WasmMesh::inner()`, and descriptor conversion; existing `belfast::Device`, `belfast::Draw`, and `belfast::DrawOptions`.
- Produces: `Device.create(canvas)`, `Device.resize()`, `Device.render(draw, mesh)`, `Device.format()`, and a JavaScript-facing `Draw` constructor.

- [ ] **Step 1: Write failing surface-size tests**

Add pure tests to `device.rs` before browser code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_css_size_by_device_pixel_ratio() {
        assert_eq!(surface_size(320, 180, 2.0), Some((640, 360)));
    }

    #[test]
    fn rounds_fractional_device_pixel_dimensions() {
        assert_eq!(surface_size(321, 181, 1.5), Some((482, 272)));
    }

    #[test]
    fn skips_zero_sized_canvas() {
        assert_eq!(surface_size(0, 180, 2.0), None);
        assert_eq!(surface_size(320, 0, 2.0), None);
    }

    #[test]
    fn normalizes_invalid_device_pixel_ratio() {
        assert_eq!(surface_size(320, 180, f64::NAN), Some((320, 180)));
        assert_eq!(surface_size(320, 180, 0.0), Some((320, 180)));
    }
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cd rust
cargo test -p belfast-wasm device::tests -- --nocapture
```

Expected: compilation fails because `device` and `surface_size` do not exist.

- [ ] **Step 3: Implement deterministic sizing**

Implement:

```rust
pub(crate) fn surface_size(
    client_width: u32,
    client_height: u32,
    device_pixel_ratio: f64,
) -> Option<(u32, u32)>;
```

Use `1.0` for non-finite or non-positive pixel ratios, multiply each CSS dimension, round to the nearest integer, clamp to at least one for non-zero CSS dimensions, and return `None` when either CSS dimension is zero.

- [ ] **Step 4: Run sizing tests and verify GREEN**

Run:

```bash
cd rust
cargo test -p belfast-wasm device::tests -- --nocapture
```

Expected: all four sizing tests pass.

- [ ] **Step 5: Implement browser device creation and resizing**

Enable these `web-sys` features in `belfast-wasm`: `HtmlCanvasElement`, `Window`.

Move `WasmDevice` into `device.rs`. Preserve `createHeadless()` for compatibility. Under `cfg(target_arch = "wasm32")`, store:

```rust
struct CanvasTarget {
    canvas: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[wasm_bindgen(js_name = Device)]
pub struct WasmDevice {
    inner: belfast::Device,
    #[cfg(target_arch = "wasm32")]
    canvas_target: Option<CanvasTarget>,
}
```

Implement `Device.create(canvas)` by creating `wgpu::SurfaceTarget::Canvas`, requesting a surface-compatible high-performance adapter, requesting the device with WebGL2-compatible downlevel limits on WASM, preferring an sRGB surface format, and configuring FIFO presentation. Construct the reusable core device with `belfast::Device::from_wgpu`.

Implement `resize()` to read `clientWidth`, `clientHeight`, and `window.devicePixelRatio`, update canvas backing dimensions only when changed, and reconfigure the surface only when dimensions changed. Return `false` for a zero-sized canvas and `true` when rendering can proceed.

- [ ] **Step 6: Implement the Draw binding and frame submission**

In `draw.rs`, deserialize an optional label from a strict camelCase options object and expose:

```rust
#[wasm_bindgen(js_name = Draw)]
pub struct WasmDraw {
    inner: belfast::Draw,
}

#[wasm_bindgen(js_class = Draw)]
impl WasmDraw {
    #[wasm_bindgen(constructor)]
    pub fn new(
        device: &WasmDevice,
        shader_code: &str,
        mesh: &WasmMesh,
        options: JsValue,
    ) -> Result<WasmDraw, JsValue>;
}
```

Add `Device.render(draw, mesh)` to acquire the current surface texture, begin one clear-color render pass using `{ r: 0.02, g: 0.025, b: 0.04, a: 1.0 }`, call `belfast::Draw::draw` with one instance, submit, and present. Reconfigure and skip a frame on `Lost` or `Outdated`, skip a frame on `Timeout`, and return a JavaScript error on `OutOfMemory`. Return an error when called on a headless device.

- [ ] **Step 7: Verify facade tests and WASM compilation**

Run:

```bash
cd rust
cargo fmt --all -- --check
cargo test -p belfast-wasm
cargo check -p belfast-wasm --target wasm32-unknown-unknown
cargo clippy -p belfast-wasm --all-targets -- -D warnings
```

Expected: all commands pass without warnings.

- [ ] **Step 8: Build and inspect generated TypeScript declarations**

Run:

```bash
cd rust
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
rg -n "create\(|fromData|addVertexBuffer|render\(|class Draw|class BufferUsage" pkg/belfast-wasm/belfast_wasm.d.ts
```

Expected: declarations expose `Device.create`, `Device.resize`, `Device.render`, `Buffer.fromData`, `Mesh.addVertexBuffer`, `Draw`, and static `BufferUsage.vertex` with camelCase names.

- [ ] **Step 9: Commit browser rendering**

```bash
git add rust/Cargo.toml rust/Cargo.lock rust/crates/belfast-wasm/Cargo.toml rust/crates/belfast-wasm/src/lib.rs rust/crates/belfast-wasm/src/device.rs rust/crates/belfast-wasm/src/draw.rs
git commit -m "feat(rust): render wasm canvas frames"
```

### Task 3: Colored Triangle Web Gallery

**Files:**

- Modify: `pnpm-workspace.yaml`
- Modify: `pnpm-lock.yaml`
- Create: `rust/web/examples/package.json`
- Create: `rust/web/examples/tsconfig.json`
- Create: `rust/web/examples/vite.config.ts`
- Create: `rust/web/examples/index.html`
- Create: `rust/web/examples/src/main.ts`
- Create: `rust/web/examples/src/style.css`
- Create: `rust/web/examples/src/examples/colored-triangle.ts`
- Create: `rust/web/examples/src/shaders/colored-triangle.wgsl`

**Interfaces:**

- Consumes: generated `rust/pkg/belfast-wasm` package and Task 2's declared API.
- Produces: the `@belfast/rust-wasm-examples` Vite package and its first selectable example module.

- [ ] **Step 1: Add the package skeleton and verify the build fails**

Add `rust/web/examples` to `pnpm-workspace.yaml`. Create `package.json` with:

```json
{
  "name": "@belfast/rust-wasm-examples",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "belfast-wasm": "file:../../pkg/belfast-wasm"
  },
  "devDependencies": {
    "typescript": "^5.7.2",
    "vite": "^6.0.3"
  }
}
```

Create `tsconfig.json` using ES2022, ESNext modules, bundler resolution, strict mode, no emit, DOM libraries, and `vite/client` types. Create a minimal Vite config and an `index.html` that loads `/src/main.ts`.

Run:

```bash
pnpm install
pnpm --filter @belfast/rust-wasm-examples typecheck
```

Expected: typecheck fails because `src/main.ts` does not exist.

- [ ] **Step 2: Add the example registry and full-viewport shell**

Create `main.ts` that imports the generated package initializer, initializes it once, selects `colored-triangle` from `URLSearchParams`, and invokes an example function with the page canvas. Unknown example names must throw `Unknown example: <name>`.

Create `style.css` with a full-viewport, non-scrolling canvas. Add a small error `<pre>` only after initialization or rendering fails; normal rendering contains no instructional UI.

Use this module contract so failures raised after the initial async setup can
reach the page shell:

```ts
export type ReportError = (error: unknown) => void;

export type WebExample = (
  canvas: HTMLCanvasElement,
  reportError: ReportError,
) => Promise<() => void>;
```

The returned cleanup function cancels the animation frame.

- [ ] **Step 3: Implement JavaScript-owned colored triangle data**

Create `colored-triangle.ts` with separate arrays:

```ts
const positions = new Float32Array([0.0, 0.6, -0.6, -0.5, 0.6, -0.5]);

const colors = new Float32Array([1.0, 0.2, 0.15, 0.15, 0.95, 0.35, 0.2, 0.4, 1.0]);
```

Create two `Buffer.fromData` calls, chain two `Mesh.addVertexBuffer` calls with the approved layouts, construct `Draw` from the imported WGSL string, and render in `requestAnimationFrame`. Catch a fatal `device.render` error inside the callback, stop scheduling frames, and pass the error to the supplied `reportError` callback.

- [ ] **Step 4: Add the WGSL shader**

Create a shader with `@location(0) position: vec2f`, `@location(1) color: vec3f`, a vertex output carrying color, and a fragment entry point returning `vec4f(input.color, 1.0)`.

- [ ] **Step 5: Verify TypeScript and production bundling**

Run:

```bash
pnpm --filter @belfast/rust-wasm-examples typecheck
pnpm --filter @belfast/rust-wasm-examples build
```

Expected: both commands pass, and Vite emits HTML, JavaScript, and the generated Belfast `.wasm` asset under `rust/web/examples/dist`.

- [ ] **Step 6: Commit the Web gallery**

```bash
git add pnpm-workspace.yaml pnpm-lock.yaml rust/web/examples
git commit -m "feat(rust): add wasm colored triangle"
```

### Task 4: Documentation And End-To-End Verification

**Files:**

- Modify: `rust/README.md`
- Modify: `rust/docs/rust-wgpu-api-parity.md`

**Interfaces:**

- Consumes: the complete generated package and Web example from Tasks 1-3.
- Produces: documented build/run commands and verified browser integration status.

- [ ] **Step 1: Document the browser workflow**

Add these commands to `rust/README.md`, preserving the rule that `wasm-pack` runs before the Web package is installed or built:

```bash
cd rust
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
cd ..
pnpm install
pnpm --filter @belfast/rust-wasm-examples dev
```

Document the gallery URL `/?example=colored-triangle` and the locations for additional example modules and shaders.

- [ ] **Step 2: Update API parity status**

Mark the WebAssembly browser smoke row implemented and record the available browser slice: `Device.create(canvas)`, vertex `Buffer.fromData`, chained `Mesh.addVertexBuffer`, `Draw`, resize, render, submit, and present. Keep texture, camera rendering, and render-to-texture browser support pending.

- [ ] **Step 3: Run the complete automated verification suite**

Run:

```bash
cd rust
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
cd ..
pnpm --filter @belfast/rust-wasm-examples typecheck
pnpm --filter @belfast/rust-wasm-examples build
```

Expected: every command exits successfully with no Rust warnings or TypeScript errors.

- [ ] **Step 4: Verify in a WebGPU browser**

Start the server:

```bash
pnpm --filter @belfast/rust-wasm-examples dev --host 127.0.0.1
```

Open `http://127.0.0.1:<port>/?example=colored-triangle`. Confirm through a screenshot and canvas pixel inspection that the canvas is nonblank, all three red/green/blue vertex regions are visible, the triangle is centered without clipping, and resizing the viewport updates the backing canvas without stretching.

- [ ] **Step 5: Commit documentation**

```bash
git add rust/README.md rust/docs/rust-wgpu-api-parity.md
git commit -m "docs(rust): document wasm web examples"
```
