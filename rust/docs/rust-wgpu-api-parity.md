# Rust wgpu API parity

Status legend:

- `Done` means the API exists and has tests or a successful build check.
- `Partial` means the name or core shape exists, but behavior is not yet equivalent to TypeScript Belfast.
- `Pending` means no Rust or WebAssembly API exists yet.

## First milestone surface

| Belfast export       | TypeScript WebGPU | Rust native | Rust WebAssembly | Notes                                                                                                                                                    |
| -------------------- | ----------------- | ----------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Device`             | Done              | Partial     | Partial          | Rust supports cloneable headless `wgpu` device creation. Wasm facade exposes `Device.createHeadless()`. Canvas/surface creation is pending.              |
| `Buffer`             | Done              | Partial     | Partial          | Rust supports create/from-data/write wrappers. Wasm class placeholder exists; JS data upload facade is pending.                                          |
| `BufferUsage`        | Done              | Partial     | Pending          | Rust exposes usage preset functions such as `BufferUsage::vertex()`. JS-style wasm constants are pending.                                                |
| `Mesh`               | Done              | Done        | Partial          | Rust supports vertex layout generation, slot assignment, index metadata, and render-pass binding. Wasm exposes constructor and read-only metadata.       |
| `Draw`               | Done              | Partial     | Partial          | Rust wraps render pipeline creation and draw calls for mesh input. Bind-group-aware draw parity is pending. Wasm class placeholder exists.               |
| `BindGroup`          | Done              | Done        | Pending          | Rust creates and binds native wgpu bind groups. A JavaScript-friendly resource descriptor facade is pending.                                             |
| `Texture`            | Done              | Partial     | Pending          | Rust uploads validated RGBA8 pixel data and exposes its view, sampler, dimensions, and format. Browser image loading is pending.                         |
| `RenderTarget`       | Done              | Partial     | Pending          | Rust supports color/depth render targets, resize, sampler, and render-pass helpers. Copy-to-screen and wasm facade are pending.                          |
| `UniformBlock`       | Done              | Done        | Partial          | Rust matches runtime schema packing, offsets, f32/u32 writes, and WGSL alignment tests. Wasm exposes a scene-uniform constructor and typed-array writes. |
| `PerspectiveCamera`  | Done              | Done        | Done             | Rust and wasm expose perspective setup, `lookAt`, aspect update, target, and view-projection output.                                                     |
| `OrthographicCamera` | Done              | Done        | Done             | Rust and wasm expose orthographic setup, `lookAt`, target, and view-projection output.                                                                   |
| `OrbitalControl`     | Done              | Done        | Pending          | Rust accepts platform-neutral pointer and scroll input; the caller owns and updates the camera.                                                          |
| `AxisHelper`         | Done              | Done        | Pending          | Rust renders RGB axes with a caller-owned camera layout and bind group.                                                                                  |
| `Geom.plane`         | Done              | Done        | Pending          | Rust has `Geom::plane` with positions, uvs, and indices. Wasm geometry facade is pending.                                                                |

## First milestone verification

| Check                         | Status  | Command                                                         |
| ----------------------------- | ------- | --------------------------------------------------------------- |
| Rust semantic/runtime tests   | Done    | `cargo test -p belfast --tests`                                 |
| WebAssembly compile check     | Done    | `cargo check -p belfast-wasm --target wasm32-unknown-unknown`   |
| Rust render target tests      | Done    | `cargo test -p belfast render_target --tests`                   |
| Native triangle examples      | Done    | `triangle` and `colored_triangle` use the shared winit harness. |
| Native camera uniform example | Done    | Uses `PerspectiveCamera`, `UniformBlock`, and `BindGroup`.      |
| Native texture example        | Done    | Uploads and samples a procedural RGBA8 checkerboard.            |
| Native render target example  | Done    | Renders offscreen and samples the target in a second pass.      |
| WebAssembly browser smoke app | Pending | Requires canvas-backed `Device.create(canvas)` facade.          |

## Next parity targets

1. Add canvas/native surface initialization to `Device`.
2. Add `BindGroup` parity so camera examples can use `UniformBlock` data in WGSL.
3. Add canvas-backed device initialization to the WebAssembly facade.
4. Add one browser smoke app that imports the generated npm package.
5. Add `CopyHelper` parity when repeated render-target presentation needs a reusable helper.
6. Expand wasm `Buffer`, `Mesh`, `Draw`, `Texture`, and `RenderTarget` into render-capable classes.
