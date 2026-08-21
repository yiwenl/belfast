# Rust wgpu API parity

Status legend:

- `Done` means the API exists and has tests or a successful build check.
- `Partial` means the name or core shape exists, but behavior is not yet equivalent to TypeScript Belfast.
- `Pending` means no Rust or WebAssembly API exists yet.

## First milestone surface

| Belfast export       | TypeScript WebGPU | Rust native | Rust WebAssembly | Notes                                                                                                                                                                                                                                                    |
| -------------------- | ----------------- | ----------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Device`             | Done              | Partial     | Partial          | Rust supports cloneable headless `wgpu` device creation. Wasm exposes `Device.createHeadless()` and canvas-backed `Device.create(canvas, { hdr? })`. `pick_surface_color` opts into `rgba16float` + extended color space when the surface advertises it. |
| `Buffer`             | Done              | Partial     | Partial          | Rust supports create/from-data/write wrappers. Wasm exposes vertex `Buffer.fromData` and the typed index upload path `Buffer.fromIndices`.                                                                                                               |
| `BufferUsage`        | Done              | Partial     | Partial          | Rust exposes usage preset functions such as `BufferUsage::vertex()`. Wasm exposes `vertex`, `uniform`, `storage`, and `vertexStorage`.                                                                                                                   |
| `Mesh`               | Done              | Done        | Partial          | Rust supports vertex layouts, slots, indices, and render-pass binding. Wasm has typed `vec2`/`vec3`/`vec4` descriptors, chained `addVertexBuffer`, instance step mode, and `setIndexBuffer`.                                                             |
| `Draw`               | Done              | Partial     | Partial          | Rust wraps render pipeline creation and draw calls for mesh input. Wasm supports browser `Draw` with optional `primitive.cullMode` and fixed-policy `depth`.                                                                                             |
| `Compute`            | Done              | Partial     | Partial          | Rust wraps compute pipeline creation and dispatch. Wasm accepts JS WGSL plus an explicit `@group(0)` buffer layout, `BindGroup.fromBuffers`, and `Frame.dispatch`.                                                                                       |
| `BindGroup`          | Done              | Done        | Partial          | Rust creates and binds native wgpu bind groups. Wasm supports draw factories plus compute `fromBuffers` for uniform/storage buffers in group 0.                                                                                                          |
| `Texture`            | Done              | Partial     | Pending          | Rust uploads validated RGBA8 pixel data and exposes its view, sampler, dimensions, and format. Browser image loading is pending.                                                                                                                         |
| `RenderTarget`       | Done              | Partial     | Partial          | Rust supports color/depth render targets, resize, sampler, and render-pass helpers. Wasm exposes color render-target creation and resize; copy-to-screen remains pending.                                                                                |
| `UniformBlock`       | Done              | Done        | Done             | Rust and wasm support ordered runtime schemas, WGSL-aligned sizing, named f32/u32/vector/mat3/mat4 writes, and direct `Buffer.write` uploads.                                                                                                            |
| `PerspectiveCamera`  | Done              | Done        | Done             | Rust and wasm expose perspective setup, `lookAt`, aspect/FOV updates, position, target, and separate view, projection, and view-projection matrices.                                                                                                     |
| `OrthographicCamera` | Done              | Done        | Done             | Rust and wasm expose orthographic setup, `lookAt`, position, target, and separate view, projection, and view-projection matrices.                                                                                                                        |
| `OrbitalControl`     | Done              | Done        | Done             | Rust accepts platform-neutral pointer and scroll input; wasm binds `listenerTarget` pointer/wheel events. Callers own the camera and must call `update(dt, camera)` each frame.                                                                          |
| `AxisHelper`         | Done              | Done        | Done             | Rust renders RGB axes with a caller-owned camera layout and bind group. Wasm creates an internal scene-uniform layout; shared `pipelineLayout` is not exposed yet.                                                                                       |
| `Geom.plane`         | Done              | Done        | Pending          | Rust has `Geom::plane` with positions, uvs, normals, and indices. No wasm facade is exposed.                                                                                                                                                             |
| `Geom.cube`          | Done              | Done        | Done             | Rust and wasm generate a 24-vertex indexed cube with per-face normals.                                                                                                                                                                                   |

## First milestone verification

| Check                                | Status | Command                                                                                                                         |
| ------------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Rust semantic/runtime tests          | Done   | `cargo test -p belfast --tests`                                                                                                 |
| WebAssembly compile check            | Done   | `cargo check -p belfast-wasm --target wasm32-unknown-unknown`                                                                   |
| Rust render target tests             | Done   | `cargo test -p belfast render_target --tests`                                                                                   |
| Native HDR display example           | Done   | `hdr_display` opts into an HDR swapchain and draws a linear 0–8 luminance ramp (`1.0` = SDR white).                             |
| Native compute example               | Done   | `compute` dispatches a compute shader into a `vertexStorage` buffer, then draws the triangle.                                   |
| Native camera uniform example        | Done   | Uses `PerspectiveCamera`, `UniformBlock`, and `BindGroup`.                                                                      |
| Native texture example               | Done   | Uploads and samples a procedural RGBA8 checkerboard.                                                                            |
| Native render target example         | Done   | Renders offscreen and samples the target in a second pass.                                                                      |
| Native examples on wasm              | Done   | Shared winit harness builds for `wasm32-unknown-unknown`; `./scripts/wasm-example.sh triangle` serves the same Rust example.    |
| WebAssembly browser smoke app        | Done   | `Device.create(canvas)`, vertex `Buffer.fromData`, chained `Mesh.addVertexBuffer`, `Draw`, resize, render, submit, and present. |
| WebAssembly camera-orbit example     | Done   | `PerspectiveCamera`, `OrbitalControl({ listenerTarget })`, and separate view/projection uniforms.                               |
| WebAssembly compute-triangle example | Done   | `Compute` with JS layout/storage buffers, `Frame.dispatch`, then `Draw` of compute-written vertices.                            |
| WebAssembly template example         | Done   | `PerspectiveCamera`, `OrbitalControl({ listenerTarget })`, and `AxisHelper` with a view-projection uniform.                     |
| WebAssembly instancing example       | Done   | Indexed `Geom.cube`, instance-rate attributes, `Frame.render(..., instanceCount)`, and canvas depth.                            |

The implemented browser slice includes the smoke app above plus camera rendering through `PerspectiveCamera`, `OrbitalControl`, `AxisHelper`, and uniform bind groups. Browser compute dispatch is available for `@group(0)` buffer layouts. Indexed instanced draws can pass `instanceCount` to `Frame.render`, and canvas passes can attach fixed `Depth24Plus`/`Less` depth with `bindTarget(..., { depth: true })`. Browser render-to-texture copy-to-screen support remains pending.

## Next parity targets

1. Add browser render-to-texture copy-to-screen support.
2. Add `CopyHelper` parity when repeated render-target presentation needs a reusable helper.
3. Expand wasm `Buffer`, `Mesh`, `Draw`, `Texture`, and `RenderTarget` beyond the current browser slice.
