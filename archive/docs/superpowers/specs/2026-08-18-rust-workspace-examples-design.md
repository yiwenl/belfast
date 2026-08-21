# Rust Workspace and Native Examples Design

## Goal

Keep the existing TypeScript Belfast package and Web examples unchanged while making the Rust implementation a self-contained, idiomatic Cargo workspace under `/rust`.

## Workspace Boundary

- `packages/belfast` remains the stable TypeScript WebGPU library.
- The existing root `examples` directory remains the TypeScript experiment gallery.
- All Rust source, Cargo metadata, Rust examples, Rust tests, Rust-specific documentation, and generated WebAssembly packaging live under `rust/`.
- The Rust implementation does not organize examples around one-to-one TypeScript parity. API parity is tracked through tests and documentation.

## Rust Packages

The nested Cargo workspace contains two crates:

- `rust/crates/belfast`: the reusable Rust renderer and `wgpu` API.
- `rust/crates/belfast-wasm`: a `wasm-bindgen` facade that translates the Rust API into JavaScript-friendly classes and errors.

The core `belfast` crate must not depend on `wasm-bindgen`, `web-sys`, or JavaScript naming conventions. Native Rust uses snake_case methods, options structs, `Result`, and explicit ownership. The facade crate owns camelCase names and browser-specific conversion.

## Native Examples

Native examples live in Cargo's conventional `rust/crates/belfast/examples/` directory and run with `cargo run -p belfast --example <name>` from `rust/`.

The initial examples are:

1. `triangle`: a constant-color triangle with a position vertex buffer.
2. `colored_triangle`: a triangle with separate position and per-vertex color buffers.
3. `camera_uniform`: a perspective camera uploaded through a uniform buffer and bind group.
4. `texture`: a textured quad using a procedurally generated RGBA checkerboard.
5. `render_to_texture`: an offscreen pass rendered into `RenderTarget`, then sampled in a second pass.

Shared native window, surface, resize, and event-loop code lives in `examples/common/mod.rs`. Shared WGSL lives in `examples/shaders/`. Windowing dependencies are dev-dependencies, so applications consuming `belfast` do not inherit `winit`.

## WebAssembly Boundary

`belfast-wasm` remains a separate crate. `wasm-pack` generates an npm package from that crate; generated output is not handwritten source. A single browser smoke app may live under `rust/web/wasm-smoke` to prove that a normal Web project can import the generated package. The five native examples are not duplicated as five Web projects.

## API Additions Required by Examples

- `Device::from_wgpu` wraps a framework-created `wgpu::Device`, `wgpu::Queue`, and target texture format.
- `BindGroup` owns a `wgpu::BindGroup`, exposes creation from a layout and entries, and can bind itself to a render pass.
- `Texture` uploads validated RGBA8 pixel data and exposes its texture view, sampler, dimensions, and format.
- Existing `Buffer`, `Mesh`, `Draw`, `UniformBlock`, cameras, and `RenderTarget` are reused.

The native example harness owns `winit` and `wgpu::Surface`; the reusable Belfast crate does not own a windowing framework.

## Verification

- Unit and integration tests cover data validation and public API behavior.
- `cargo test --workspace --all-targets` compiles all examples and runs tests.
- `cargo clippy --workspace --all-targets -- -D warnings` covers libraries, tests, and examples.
- `cargo check -p belfast-wasm --target wasm32-unknown-unknown` verifies the WebAssembly facade.
- Each native example is manually launched to confirm surface creation and pipeline validation where the environment permits opening windows.
