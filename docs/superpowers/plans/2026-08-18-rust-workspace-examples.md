# Rust Workspace and Native Examples Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Belfast Rust into an isolated `/rust` Cargo workspace and add five runnable native `wgpu` examples.

**Architecture:** The existing TypeScript library and examples remain unchanged. The nested Rust workspace contains a reusable `belfast` crate, a separate `belfast-wasm` facade, Cargo-native examples owned by the core crate, and a shared `winit` harness that stays in dev-only code.

**Tech Stack:** Rust 2021, `wgpu` 0.20, `winit` 0.30, `pollster` 0.4, `glam` 0.30, `bytemuck` 1.25, `wasm-bindgen` 0.2.

## Global Constraints

- Keep `packages/belfast` and the root `examples` directory behavior unchanged.
- Put all Cargo workspace files, Rust sources, Rust tests, and Rust examples under `rust/`.
- Keep `belfast` free of runtime dependencies on `winit`, `wasm-bindgen`, `js-sys`, and `web-sys`.
- Keep `belfast-wasm` as a separate facade crate.
- Use standard Cargo examples instead of one crate per example.
- Do not duplicate the five native examples as five Web projects.
- Preserve all existing uncommitted render-target behavior and tests during the move.

---

### Task 1: Isolate the Cargo workspace under `/rust`

**Files:**

- Move: `Cargo.toml` to `rust/Cargo.toml`
- Move: `Cargo.lock` to `rust/Cargo.lock`
- Move: `crates/belfast` to `rust/crates/belfast`
- Move: `crates/belfast-wasm` to `rust/crates/belfast-wasm`
- Move: `docs/rust` to `rust/docs`
- Create: `rust/README.md`
- Modify: `.gitignore`
- Modify: `packages/belfast-wasm/package.json`

**Interfaces:**

- Produces: a nested Cargo workspace invoked with `cargo --manifest-path rust/Cargo.toml` from the repository root or ordinary `cargo` commands from `rust/`.

- [x] Move the existing workspace and Rust documentation without changing Rust source behavior.
- [x] Change the `belfast-wasm` build script to use `../../rust/crates/belfast-wasm` until the generated npm package is fully relocated.
- [x] Add `rust/pkg/` to `.gitignore` for future direct `wasm-pack` output.
- [x] Run `cargo test --manifest-path rust/Cargo.toml -p belfast --tests` and expect all existing tests to pass.

### Task 2: Add framework-independent device wrapping

**Files:**

- Modify: `rust/crates/belfast/src/device.rs`
- Test: `rust/crates/belfast/tests/device.rs`

**Interfaces:**

- Produces: `pub fn Device::from_wgpu(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Device`.
- Consumed by: the native `winit` example harness.

- [x] Add a compile-time integration test that constructs a headless device, extracts no private state, and verifies the public format contract.
- [x] Add `Device::from_wgpu` and make existing headless creation delegate to it.
- [x] Run `cargo test -p belfast --test device` from `rust/` and expect PASS.

### Task 3: Add bind-group and RGBA texture APIs

**Files:**

- Create: `rust/crates/belfast/src/bind_group.rs`
- Create: `rust/crates/belfast/src/texture.rs`
- Modify: `rust/crates/belfast/src/lib.rs`
- Modify: `rust/crates/belfast/src/error.rs`
- Test: `rust/crates/belfast/tests/texture.rs`

**Interfaces:**

- Produces: `BindGroup::create(&Device, &wgpu::BindGroupLayout, &[wgpu::BindGroupEntry<'_>], &str) -> BindGroup`.
- Produces: `BindGroup::bind(&self, &mut wgpu::RenderPass<'_>, u32)` and `BindGroup::gpu(&self) -> &wgpu::BindGroup`.
- Produces: `Texture::from_rgba8(&Device, u32, u32, &[u8], TextureOptions) -> BelfastResult<Texture>`.
- Produces: `Texture::{view, sampler, width, height, format}` accessors.

- [x] Write tests asserting zero dimensions and incorrect RGBA byte lengths return explicit errors.
- [x] Run `cargo test -p belfast --test texture` and verify the tests fail because `Texture` and its errors do not exist.
- [x] Implement validation, texture upload through `Queue::write_texture`, a default linear clamp sampler, and bind-group ownership.
- [x] Run the texture test and all existing tests; expect PASS.

### Task 4: Add the shared native window and surface harness

**Files:**

- Modify: `rust/crates/belfast/Cargo.toml`
- Create: `rust/crates/belfast/examples/common/mod.rs`

**Interfaces:**

- Produces: an `Example` trait with `new`, `resize`, and `render` lifecycle methods.
- Produces: `common::run::<E>(title)` that owns the `winit` application lifecycle, `wgpu::Surface`, surface configuration, and Belfast `Device`.
- Produces: `ExampleContext` access to `device`, `format`, `width`, and `height`.

- [x] Add `winit = "0.30"` and `pollster = "0.4"` as dev-dependencies.
- [x] Implement surface creation from an `Arc<Window>`, choose an sRGB surface format when available, configure on non-zero resize, and recover from `SurfaceError::Lost` or `Outdated` by reconfiguring.
- [x] Run `cargo check -p belfast --examples` and expect the common harness to compile through its first consumer in Task 5.

### Task 5: Add triangle examples

**Files:**

- Create: `rust/crates/belfast/examples/triangle.rs`
- Create: `rust/crates/belfast/examples/colored_triangle.rs`
- Create: `rust/crates/belfast/examples/shaders/triangle.wgsl`
- Create: `rust/crates/belfast/examples/shaders/colored_triangle.wgsl`

**Interfaces:**

- Consumes: `Buffer`, `Mesh`, `Draw`, `Example`, and `ExampleContext`.
- Produces: `cargo run -p belfast --example triangle` and `cargo run -p belfast --example colored_triangle`.

- [x] Implement the constant-color triangle with one position buffer and a three-vertex mesh.
- [x] Implement the colored triangle with separate position and RGB vertex buffers in slots 0 and 1.
- [x] Compile both examples with `cargo check -p belfast --example triangle --example colored_triangle` and expect PASS.

### Task 6: Add the camera uniform example

**Files:**

- Create: `rust/crates/belfast/examples/camera_uniform.rs`
- Create: `rust/crates/belfast/examples/shaders/camera_uniform.wgsl`

**Interfaces:**

- Consumes: `PerspectiveCamera`, `UniformBlock`, `Buffer`, `BindGroup`, and `Draw`.
- Produces: `cargo run -p belfast --example camera_uniform`.

- [x] Create a `view_proj: mat4x4<f32>` uniform schema, GPU uniform buffer, explicit bind-group layout, pipeline layout, and bind group.
- [x] Update camera aspect on resize.
- [x] Upload the camera matrix and bind group before drawing each frame.
- [x] Run `cargo check -p belfast --example camera_uniform` and expect PASS.

### Task 7: Add the texture example

**Files:**

- Create: `rust/crates/belfast/examples/texture.rs`
- Create: `rust/crates/belfast/examples/shaders/texture.wgsl`

**Interfaces:**

- Consumes: `Texture::from_rgba8`, `Geom::plane`, indexed `Mesh`, `BindGroup`, and `Draw`.
- Produces: `cargo run -p belfast --example texture`.

- [x] Generate a small RGBA checkerboard in Rust so the example has no filesystem or image-decoder dependency.
- [x] Upload positions, UVs, and indices; create texture and sampler bindings; render the indexed quad.
- [x] Run `cargo check -p belfast --example texture` and expect PASS.

### Task 8: Add the render-to-texture example

**Files:**

- Create: `rust/crates/belfast/examples/render_to_texture.rs`
- Create: `rust/crates/belfast/examples/shaders/render_target_source.wgsl`
- Create: `rust/crates/belfast/examples/shaders/render_target_present.wgsl`

**Interfaces:**

- Consumes: `RenderTarget`, `RenderPassOptions`, `BindGroup`, two `Draw` pipelines, and fullscreen triangle geometry.
- Produces: `cargo run -p belfast --example render_to_texture`.

- [x] Render an animated colored triangle into a surface-format `RenderTarget` in the first pass.
- [x] Sample `RenderTarget::color_view()` and `RenderTarget::sampler()` through a bind group in the second pass.
- [x] Resize the offscreen target with the surface and submit both passes in one command encoder.
- [x] Run `cargo check -p belfast --example render_to_texture` and expect PASS.

### Task 9: Reconnect WebAssembly and document commands

**Files:**

- Modify: `rust/crates/belfast-wasm/Cargo.toml`
- Modify: `rust/README.md`
- Modify: `rust/docs/rust-wgpu-api-parity.md`
- Delete: `packages/belfast-wasm/package.json`
- Modify: `pnpm-workspace.yaml`

**Interfaces:**

- Produces: documented native example commands and a compiling `wasm32-unknown-unknown` facade.

- [x] Update Rust documentation paths and mark the five native examples complete.
- [x] Generate the npm package directly into ignored `rust/pkg/belfast-wasm` output with `wasm-pack`.
- [x] Run `cargo fmt --manifest-path rust/Cargo.toml --all -- --check`.
- [x] Run `cargo test --manifest-path rust/Cargo.toml --workspace --all-targets`.
- [x] Run `cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`.
- [x] Run `cargo check --manifest-path rust/Cargo.toml -p belfast-wasm --target wasm32-unknown-unknown`.
- [x] Launch each native example long enough to verify surface and pipeline creation, then close it cleanly.
