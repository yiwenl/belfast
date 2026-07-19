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
| `UniformBlock`       | Done              | Done        | Partial          | Rust matches runtime schema packing, offsets, f32/u32 writes, and WGSL alignment tests. Wasm exposes a scene-uniform constructor and typed-array writes. |
| `PerspectiveCamera`  | Done              | Done        | Done             | Rust and wasm expose perspective setup, `lookAt`, aspect update, target, and view-projection output.                                                     |
| `OrthographicCamera` | Done              | Done        | Done             | Rust and wasm expose orthographic setup, `lookAt`, target, and view-projection output.                                                                   |
| `Geom.plane`         | Done              | Done        | Pending          | Rust has `Geom::plane` with positions, uvs, and indices. Wasm geometry facade is pending.                                                                |

## First milestone verification

| Check                          | Status  | Command                                                         |
| ------------------------------ | ------- | --------------------------------------------------------------- |
| Rust semantic/runtime tests    | Done    | `cargo test -p belfast --tests`                                 |
| WebAssembly compile check      | Done    | `cargo check -p belfast-wasm --target wasm32-unknown-unknown`   |
| Native triangle example        | Pending | Requires window/surface or headless render example wiring.      |
| Wasm triangle example          | Pending | Requires canvas-backed `Device.create(canvas)` facade.          |
| Native camera-triangle example | Pending | Requires bind group facade and example wiring.                  |
| Wasm camera-triangle example   | Pending | Requires canvas-backed device, bind groups, and example wiring. |

## Next parity targets

1. Add canvas/native surface initialization to `Device`.
2. Add `BindGroup` parity so camera examples can use `UniformBlock` data in WGSL.
3. Add paired `triangle` native and wasm examples that share existing WGSL shader code.
4. Add paired `camera-triangle` native and wasm examples once bind groups are available.
5. Expand wasm `Buffer`, `Mesh`, and `Draw` from placeholders/read-only wrappers into render-capable classes.
