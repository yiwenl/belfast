# Rust wgpu API parity

Status legend:

- `Done` means the API exists and has tests or a successful build check.
- `Partial` means the name or core shape exists, but behavior is not yet equivalent to TypeScript Belfast.
- `Pending` means no Rust or WebAssembly API exists yet.

## First milestone surface

| Belfast export       | TypeScript WebGPU | Rust native | Rust WebAssembly | Notes                                                                                                                                                                           |
| -------------------- | ----------------- | ----------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Device`             | Done              | Partial     | Partial          | Rust supports cloneable headless `wgpu` device creation. Wasm exposes `Device.createHeadless()` and canvas-backed `Device.create(canvas)`.                                      |
| `Buffer`             | Done              | Partial     | Partial          | Rust supports create/from-data/write wrappers. Wasm exposes vertex `Buffer.fromData` for the browser smoke app; broader JS data upload parity is pending.                       |
| `BufferUsage`        | Done              | Partial     | Partial          | Rust exposes usage preset functions such as `BufferUsage::vertex()`. Wasm exposes `vertex`, `uniform`, `storage`, and `vertexStorage`.                                          |
| `Mesh`               | Done              | Done        | Partial          | Rust supports vertex layouts, slots, indices, and render-pass binding. Wasm has typed `vec2`/`vec3`/`vec4` descriptors and chained `addVertexBuffer`; indices remain pending.   |
| `Draw`               | Done              | Partial     | Partial          | Rust wraps render pipeline creation and draw calls for mesh input. Wasm supports the browser smoke `Draw` path; bind-group-aware draw parity is pending.                        |
| `Compute`            | Done              | Partial     | Partial          | Rust wraps compute pipeline creation and dispatch. Wasm accepts JS WGSL plus an explicit `@group(0)` buffer layout, `BindGroup.fromBuffers`, and `Frame.dispatch`.              |
| `BindGroup`          | Done              | Done        | Partial          | Rust creates and binds native wgpu bind groups. Wasm supports draw factories plus compute `fromBuffers` for uniform/storage buffers in group 0.                                 |
| `Texture`            | Done              | Partial     | Pending          | Rust uploads validated RGBA8 pixel data and exposes its view, sampler, dimensions, and format. Browser image loading is pending.                                                |
| `RenderTarget`       | Done              | Partial     | Pending          | Rust supports color/depth render targets, resize, sampler, and render-pass helpers. Copy-to-screen and wasm facade are pending.                                                 |
| `UniformBlock`       | Done              | Done        | Done             | Rust and wasm support ordered runtime schemas, WGSL-aligned sizing, named f32/u32/vector/mat3/mat4 writes, and direct `Buffer.write` uploads.                                   |
| `PerspectiveCamera`  | Done              | Done        | Done             | Rust and wasm expose perspective setup, `lookAt`, aspect/FOV updates, position, target, and separate view, projection, and view-projection matrices.                            |
| `OrthographicCamera` | Done              | Done        | Done             | Rust and wasm expose orthographic setup, `lookAt`, position, target, and separate view, projection, and view-projection matrices.                                               |
| `OrbitalControl`     | Done              | Done        | Done             | Rust accepts platform-neutral pointer and scroll input; wasm binds `listenerTarget` pointer/wheel events. Callers own the camera and must call `update(dt, camera)` each frame. |
| `AxisHelper`         | Done              | Done        | Done             | Rust renders RGB axes with a caller-owned camera layout and bind group. Wasm creates an internal scene-uniform layout; shared `pipelineLayout` is not exposed yet.              |
| `Geom.plane`         | Done              | Done        | Pending          | Rust has `Geom::plane` with positions, uvs, and indices. Wasm geometry facade is pending.                                                                                       |

## First milestone verification

| Check                                | Status | Command                                                                                                                         |
| ------------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Rust semantic/runtime tests          | Done   | `cargo test -p belfast --tests`                                                                                                 |
| WebAssembly compile check            | Done   | `cargo check -p belfast-wasm --target wasm32-unknown-unknown`                                                                   |
| Rust render target tests             | Done   | `cargo test -p belfast render_target --tests`                                                                                   |
| Native triangle examples             | Done   | `triangle` and `colored_triangle` use the shared winit harness.                                                                 |
| Native camera uniform example        | Done   | Uses `PerspectiveCamera`, `UniformBlock`, and `BindGroup`.                                                                      |
| Native texture example               | Done   | Uploads and samples a procedural RGBA8 checkerboard.                                                                            |
| Native render target example         | Done   | Renders offscreen and samples the target in a second pass.                                                                      |
| WebAssembly browser smoke app        | Done   | `Device.create(canvas)`, vertex `Buffer.fromData`, chained `Mesh.addVertexBuffer`, `Draw`, resize, render, submit, and present. |
| WebAssembly camera-orbit example     | Done   | `PerspectiveCamera`, `OrbitalControl({ listenerTarget })`, and separate view/projection uniforms.                               |
| WebAssembly compute-triangle example | Done   | `Compute` with JS layout/storage buffers, `Frame.dispatch`, then `Draw` of compute-written vertices.                            |
| WebAssembly template example         | Done   | `PerspectiveCamera`, `OrbitalControl({ listenerTarget })`, and `AxisHelper` with a view-projection uniform.                     |

The implemented browser slice includes the smoke app above plus camera rendering through `PerspectiveCamera`, `OrbitalControl`, `AxisHelper`, and uniform bind groups. Browser compute dispatch is available for `@group(0)` buffer layouts. Browser render-to-texture support remains pending.

## Next parity targets

1. Add browser render-to-texture support.
2. Add `CopyHelper` parity when repeated render-target presentation needs a reusable helper.
3. Expand wasm `Buffer`, `Mesh`, `Draw`, `Texture`, and `RenderTarget` beyond the browser smoke slice.
4. Add `Geom.plane` wasm facade.
