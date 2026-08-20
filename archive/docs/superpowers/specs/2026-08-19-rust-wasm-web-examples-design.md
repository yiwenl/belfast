# Rust WASM Web Examples Design

## Goal

Add a browser example gallery under `rust/web/examples` and use its first
example to prove that JavaScript can provide vertex positions, vertex colors,
and WGSL to the Rust Belfast renderer compiled to WebAssembly.

The first example is a colored triangle. It is also the first end-to-end slice
through the JavaScript-facing `Device`, `Buffer`, `Mesh`, and `Draw` classes.

## Project Boundary

- Native Rust examples remain in `rust/crates/belfast/examples`.
- The `belfast-wasm` crate remains a browser-specific `wasm-bindgen` facade.
- `wasm-pack` continues to write generated npm output to
  `rust/pkg/belfast-wasm`; generated files are not handwritten or committed.
- Browser examples live in one Vite project at `rust/web/examples` instead of
  inside `rust/crates/belfast-wasm/examples`.
- Additional browser experiments are added as modules within this Vite project,
  so they share package installation, build configuration, and example
  navigation.

Initial layout:

```text
rust/web/examples/
├── package.json
├── index.html
├── tsconfig.json
├── vite.config.ts
└── src/
    ├── main.ts
    ├── examples/
    │   └── colored-triangle.ts
    └── shaders/
        └── colored-triangle.wgsl
```

## Browser API Boundary

The WebAssembly facade preserves the Belfast class vocabulary and JavaScript
naming where practical:

- `Device.create(canvas)` creates a surface-compatible `wgpu` device and owns
  the browser surface configuration.
- `BufferUsage.vertex` identifies vertex-buffer usage.
- `Buffer.fromData(device, values, usage, label)` copies a JavaScript
  `Float32Array` into a Rust-owned `wgpu::Buffer`.
- `Mesh.addVertexBuffer(buffer, descriptor)` associates a Rust buffer with its
  vertex layout. Passing the buffer separately keeps the `wasm-bindgen` class
  boundary typed while preserving the original descriptor vocabulary.
- `Draw` creates a Rust `wgpu::RenderPipeline` from WGSL supplied by JavaScript
  and the mesh's vertex layouts.
- `Device.render(draw, mesh)` acquires the current surface texture, records the
  render pass, submits it, and presents the frame.

Raw `wgpu::CommandEncoder`, `wgpu::RenderPass`, and WebGPU handles do not cross
the WebAssembly boundary. This differs from the TypeScript Belfast render loop
because those Rust resources have lifetimes and ownership rules that cannot be
represented safely as ordinary JavaScript objects. The facade keeps the same
high-level classes while making command recording a Rust-owned operation.

The API is pre-1.0. The first browser slice replaces existing placeholder
bindings where required by this flow, but it must not introduce a
colored-triangle-specific renderer or other example-only production API.

## Colored Triangle Data Flow

JavaScript creates two separate arrays, matching the existing TypeScript and
native Rust examples:

- positions: three `float32x2` vertices;
- colors: three `float32x3` values.

The arrays cross the WebAssembly boundary only during `Buffer.fromData`.
`belfast-wasm` validates usage and layout descriptors, then delegates buffer,
mesh, and pipeline creation to the reusable `belfast` crate. The animation loop
calls `device.resize()` and `device.render(draw, mesh)`; GPU resource ownership
remains in Rust between frames.

The WGSL source belongs to the browser example and is imported as text by Vite.
This proves that shader experiments can remain editable Web-project assets
rather than being compiled into the Rust facade.

## Surface And Resize Behavior

`Device.create(canvas)` requests an adapter compatible with the canvas surface,
chooses an sRGB surface format when available, creates the Rust `Device`, and
configures the surface using the canvas's displayed size and device pixel
ratio.

`device.resize()` updates the canvas backing dimensions and reconfigures the
surface only when dimensions change. Zero-sized canvases skip rendering.
Recoverable lost or outdated surface errors trigger reconfiguration; timeout
errors skip the frame; out-of-memory errors are surfaced to JavaScript.

## Error Handling

- Async initialization rejects with a JavaScript `Error` when WebGPU, a
  compatible adapter, or device creation is unavailable.
- Invalid usage names, vertex formats, empty arrays, malformed attributes,
  duplicate slots, and inconsistent layout sizes return JavaScript errors.
- Surface acquisition and rendering failures are returned from
  `Device.render`; the example logs the error and stops its animation loop when
  the error is fatal.
- Rust panics are not used for user-provided JavaScript data.

## Testing And Verification

- Rust unit tests cover JavaScript-facing descriptor conversion and validation
  where the logic can be tested without a browser.
- `cargo test --workspace --all-targets` verifies the Rust workspace and native
  examples.
- `cargo clippy --workspace --all-targets -- -D warnings` checks Rust code.
- `cargo check -p belfast-wasm --target wasm32-unknown-unknown` verifies the
  facade target.
- `wasm-pack build crates/belfast-wasm --target web` regenerates the npm
  package consumed by the example project.
- The Vite project must pass TypeScript typechecking and a production build.
- The development server is opened in a WebGPU-capable browser to confirm the
  triangle renders with three distinct vertex colors and remains correctly
  sized after a viewport resize.

## Documentation

`rust/README.md` documents how to build the WebAssembly package and run the Web
examples. The Rust API parity document records the browser surface and colored
triangle slice as implemented once verification passes.
