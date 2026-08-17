# PRD: Rust wgpu Belfast Rendering Engine

## Problem Statement

Belfast currently provides a TypeScript WebGPU library that is already used by multiple Web experiments. The existing package should remain stable because those experiments depend on it. At the same time, Belfast needs a Rust implementation built on `wgpu` so the engine can eventually support native Rust applications, compile to WebAssembly for Web projects, and potentially replace or back the JavaScript implementation in the future.

The challenge is to add the Rust implementation without disrupting the existing WebGPU library, while keeping the public Belfast concepts and class-shaped API familiar enough that existing examples and mental models remain useful.

## Solution

Add a parallel Rust implementation of Belfast with a layered architecture:

- Keep the existing TypeScript WebGPU package as the current stable Web runtime.
- Add a Rust engine crate that exposes Belfast concepts using `wgpu`.
- Add a WebAssembly facade that wraps the Rust engine in JavaScript-friendly classes.
- Keep API names, concepts, and examples aligned across TypeScript, Rust native, and Rust WebAssembly.
- Track implementation parity with the existing public API so the Rust version grows deliberately instead of becoming a separate engine.

The Rust implementation should start as a parallel package rather than a drop-in replacement. Once enough API parity and example coverage exists, the project can decide whether the main JavaScript package should remain independent, wrap the WebAssembly runtime, or expose both runtimes.

## User Stories

1. As an existing Belfast Web experiment author, I want the current TypeScript WebGPU package to keep working, so that my existing experiments do not break while the Rust engine is developed.
2. As a Belfast maintainer, I want a Rust implementation built on `wgpu`, so that Belfast can target both native and WebAssembly runtimes.
3. As a Rust application author, I want to import Belfast as a Rust crate, so that I can build GPU applications without depending on JavaScript.
4. As a Web application author, I want to import a WebAssembly Belfast package from JavaScript or TypeScript, so that I can use the Rust renderer in a Vite or browser project.
5. As a Belfast user, I want the Rust WebAssembly API to feel similar to the current TypeScript API, so that examples and existing knowledge transfer cleanly.
6. As a Belfast user, I want the Rust native API to use the same engine concepts, so that moving between Web and native targets does not require relearning the engine.
7. As a Rust developer, I want the Rust API to remain idiomatic where ownership, errors, async setup, and resource lifetimes matter, so that native Rust usage remains pleasant and safe.
8. As a Web developer, I want the WebAssembly facade to expose class-shaped APIs, so that it feels natural from JavaScript even if the Rust internals are idiomatic.
9. As a maintainer, I want the Rust implementation split into semantic and backend layers, so that math, camera, geometry, uniform packing, and descriptors can stay independent from surface-specific GPU setup.
10. As a maintainer, I want `Device` setup to support browser canvas, native surface, and headless modes, so that the same engine can serve Web, desktop, and tests.
11. As a renderer user, I want `Buffer`, `Mesh`, `Draw`, `Texture`, `RenderTarget`, `BindGroup`, and `Compute` concepts to match Belfast's existing mental model, so that render loops remain recognizable.
12. As a shader author, I want WGSL entry point conventions to stay consistent, so that shaders can move between examples with minimal changes.
13. As a user working with uniforms, I want `UniformBlock` layout behavior to match the existing TypeScript implementation, so that WGSL alignment is predictable across runtimes.
14. As a user working with cameras, I want perspective and orthographic cameras to behave consistently across TypeScript, Rust native, and WebAssembly, so that scene math remains portable.
15. As a user working with geometry helpers, I want generated plane, sphere, cube, and billboard geometry to match Belfast conventions, so that examples render the same shapes across runtimes.
16. As a user working with textures, I want Rust core APIs that accept decoded pixel data and WebAssembly APIs that can load browser images, so that each runtime handles assets naturally.
17. As a user working with compute shaders, I want storage buffers and compute dispatch to be available in Rust, so that particle and simulation experiments can migrate later.
18. As a user working with shadows, I want depth rendering and shadow map helpers to eventually exist in Rust, so that existing advanced examples have a migration path.
19. As a maintainer, I want a parity matrix for the public API, so that implementation status is visible and feature work can be prioritized.
20. As a maintainer, I want idiomatic Cargo examples for the Rust renderer and one WebAssembly integration smoke app, so that native usage stays easy to learn without duplicating every example across runtimes.
21. As a maintainer, I want the first milestone to focus on a small rendering path, so that the Rust architecture can be validated before porting every helper.
22. As a maintainer, I want existing documentation to remain accurate for the TypeScript package, so that users are not confused about which runtime a feature belongs to.
23. As a future migrator, I want the JavaScript package to have a possible migration path toward a WebAssembly-backed runtime, so that Belfast can evolve without a breaking rewrite.
24. As a contributor, I want clear out-of-scope boundaries for the first phase, so that work stays focused on core engine architecture rather than every helper at once.
25. As a maintainer, I want Rust and WebAssembly build checks in CI eventually, so that both runtimes remain healthy as the API surface grows.

## Implementation Decisions

- The existing TypeScript WebGPU package remains the stable runtime and should not be replaced during the initial Rust implementation.
- The Rust implementation is introduced as a parallel runtime with its own crate and WebAssembly package.
- Rust code lives in a Cargo workspace under `crates/` with two crates: `belfast` (semantic layer and `wgpu` runtime layer as modules of one crate) and `belfast-wasm` (the WebAssembly facade).
- The Rust crate is named `belfast` and the WebAssembly npm package is named `belfast-wasm`.
- The architecture is split into a semantic layer, a `wgpu` runtime layer, and a WebAssembly facade layer.
- Math uses `glam`, which matches WGSL memory layout and works naturally with `bytemuck` for GPU upload. Matrices remain column-major, consistent with the gl-matrix behavior in the TypeScript package.
- `Device` is cheaply cloneable (its internals are shared via `Arc`), and resources store a `Device` clone at creation, mirroring the TypeScript ergonomics where resources are constructed from a device and keep using it.
- Constructor options use plain options structs with `Default` implementations and struct-update syntax, the closest Rust equivalent to the TypeScript options-object convention.
- Fallible operations (device initialization, invalid schemas, invalid input) return `Result` with a `thiserror`-based error type. Panics are reserved for programmer bugs. The WebAssembly facade converts `Result` errors into thrown JavaScript errors.
- Device creation is `async` in all runtimes. Native examples drive it with `pollster::block_on`; the WebAssembly facade exposes awaitable constructors via `wasm-bindgen-futures`, mirroring the TypeScript `await Device.create()` shape.
- The engine crate does not own windowing. `Device` accepts surface targets through `raw-window-handle`, and `winit` appears only as a dev-dependency of native examples.
- Headless device creation (no surface, render to texture) is part of the first milestone because it is cheap once native initialization exists and unlocks local GPU smoke tests.
- The browser build targets wgpu's WebGPU backend only, matching the WebGPU-only TypeScript package. There is no WebGL2 fallback, which keeps WGSL conventions and a single code path.
- The WebAssembly package is built with `wasm-pack` and wrapped in a pnpm workspace package so Vite examples consume it like any other dependency.
- The WebAssembly facade mirrors the TypeScript API surface exactly: camelCase method names, matching class and method names, and matching option shapes (via `wasm_bindgen(js_name)` where needed), so examples are near copy-paste between runtimes.
- The semantic layer owns runtime-neutral concepts such as cameras, geometry data, uniform packing rules, render state descriptors, and shared option types.
- The `wgpu` runtime layer owns GPU resources such as device, queue, surface, buffers, textures, bind groups, render targets, render pipelines, compute pipelines, and render passes.
- The WebAssembly facade wraps the Rust runtime in JavaScript-friendly classes and async constructors.
- Rust native APIs may be more idiomatic than JavaScript APIs, but exported names and concepts should stay aligned with Belfast's public API.
- The WebAssembly API should be closer to the existing TypeScript class API than the native Rust API.
- `Device` should support browser canvas creation, native surface creation, and headless creation as separate initialization paths.
- `Texture` loading should be split by responsibility: core Rust should accept decoded data, while WebAssembly can provide browser-oriented image loading helpers.
- `OrbitalControl` should not be part of the earliest Rust core milestone because it is coupled to browser and window event systems. It can be added later through input adapters.
- `UniformBlock` should be an early migration target because layout consistency is central to shader compatibility.
- `UniformBlock` in Rust uses the same runtime-schema design as TypeScript: string-keyed fields with layout computed at runtime, so schemas, offsets, and set-by-name behavior match across runtimes. A derive-macro alternative is a possible later addition, not part of this design.
- The complete Cargo workspace lives under `rust/`, while the existing TypeScript package and root Web examples remain unchanged.
- Native examples use Cargo's conventional `crates/belfast/examples/` targets and embed Rust-owned WGSL with `include_str!`.
- WebAssembly integration uses one browser smoke app against the generated `belfast-wasm` npm package instead of duplicating every native example as a Web project.
- `Mesh` should continue to represent vertex and index buffer bindings, while `Draw` should own pipeline creation and drawing behavior.
- `Draw` should preserve Belfast's WGSL conventions where possible, including expected vertex and fragment entry points.
- The initial milestone should include enough rendering capability to draw simple geometry in native Rust and compile the facade for WebAssembly.
- A public API parity matrix should be maintained as part of the feature documentation: a hand-maintained markdown table in `docs/rust/` with one row per public export of `packages/belfast/src/index.ts` and columns for TypeScript, Rust native, WebAssembly, and notes.
- Examples should act as executable specifications for each runtime.
- The eventual replacement strategy for the JavaScript package remains a later decision. The initial design only creates the path for that decision.

## Testing Decisions

- Tests should focus on external behavior and cross-runtime parity rather than private implementation details.
- The highest-value test seam is example behavior: simple examples should compile and render for the TypeScript package, Rust native runtime, and Rust WebAssembly runtime.
- Uniform layout tests should verify byte sizes, offsets, padding, scalar values, vector values, matrix values, and unsigned integer handling against WGSL-compatible expectations.
- Geometry tests should verify generated vertex counts, index counts, attribute layouts, bounds, and winding where applicable.
- Camera tests should verify view, projection, and view-projection matrix behavior for perspective and orthographic cameras.
- Mesh tests should verify vertex layout generation, slot assignment, index buffer metadata, and invalid input behavior.
- Draw and pipeline tests should start with compile-time and smoke-render coverage rather than brittle internal pipeline assertions.
- WebAssembly tests should verify that the package initializes, constructs core classes, accepts a canvas, and can drive a minimal render loop from JavaScript.
- Rust native tests should verify that the crate compiles for native targets and can create headless resources where the platform permits.
- CI gains a Rust job in the first milestone: `cargo fmt`, `cargo clippy`, `cargo test` (CPU-only semantic-layer tests), and a `wasm-pack` build check. GPU smoke tests (headless render-to-texture with pixel assertions) run locally only for now; software-adapter CI is a later decision.
- Prior art exists in the current API tests and examples, especially the triangle, texture, camera, render-to-texture, compute particles, and shadow examples.
- The parity matrix should be treated as a testing guide: each feature marked complete should have either unit coverage, example coverage, or both.

## Out of Scope

- Replacing the existing TypeScript package during the first milestone.
- Porting every helper, postprocessing pass, particle experiment, and advanced example immediately.
- Designing a full scene graph or material system.
- Rewriting existing examples to depend on Rust WebAssembly by default.
- Guaranteeing a drop-in replacement package before API parity is proven.
- Browser-specific input controls in the first Rust core milestone.
- Asset pipelines beyond basic texture upload and simple example loading.
- Publishing to crates.io or npm before the architecture and build flow are validated.

## Further Notes

The first practical milestone should be intentionally small: create the isolated Rust workspace, add the core `wgpu` runtime, expose a minimal WebAssembly facade, and prove the native rendering path with standard Cargo examples. The first examples exercise `Device`, `Buffer`, `Mesh`, `Draw`, `UniformBlock`, cameras, textures, bind groups, and render targets. A single browser smoke app validates WebAssembly package integration separately.

The recommended early API surface is `Device`, `Buffer`, `Mesh`, `Draw`, `UniformBlock`, `PerspectiveCamera`, `OrthographicCamera`, and simple geometry helpers. Once that path works, Belfast can expand toward textures, render targets, compute, depth rendering, shadow maps, and postprocessing.

The most important long-term guardrail is API parity. The Rust implementation should feel like Belfast, not like a disconnected renderer that happens to live in the same repository.
