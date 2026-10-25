# Rust Experiment Template Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task.

**Goal:** A copyable Rust/Wasm experiment with a Vite/TypeScript host, axes, mouse orbit controls, and a future Kinetic input boundary.

**Architecture:** Keep the complete experiment in `experiments/template/`. Rust owns the wgpu surface, AxisHelper, PerspectiveCamera, OrbitalControl, frame update, and skeleton state. TypeScript owns DOM events, animation scheduling, and the optional Kinetic adapter. Use the existing Belfast Rust crate directly; no scene orchestration in JS.

**Tech Stack:** Existing workspace Rust/wgpu/wasm-bindgen versions, wasm-pack, Vite 6, TypeScript 5.

**Spec:** The user's approved in-chat architecture and request for axes/orbit only, with a reserved Kinetic API.

## Global Constraints

- No particles or camera permission request in this template.
- Input payload: 33 fixed MediaPipe landmarks × `[x, y, z, confidence]`, scene coordinates, milliseconds from `performance.now()`.
- Left drag rotates; Shift-left drag or middle drag pans; wheel zooms.
- Preserve unrelated working changes; implement in this checkout with new experiment files.
- The user's implementation request authorizes completing the template without another design/plan permission round.

## Review Focus

- Pointer capture cancellation and window blur must end dragging.
- Hidden/zero-sized canvas must skip rendering safely; resize must preserve aspect and account for DPR.
- Invalid/stale skeleton payloads must not corrupt the last accepted frame; tracking expires after 250 ms.
- Hot reload must stop RAF, remove listeners, and free Rust resources, including pending initialization.
- Unsupported WebGPU or GPU failures must produce a visible error.

### Task 1: Rust experiment

**Files:** `experiments/template/Cargo.toml`, `src/{lib,input,scene,browser}.rs`, root `Cargo.toml`.

**Interfaces:** `ExperimentApp.create(canvas)`, `resize(width,height,dpr)`, `frame(timestampMs)`, pointer methods, `scroll(delta)`, `setSkeleton(Float32Array,timestampMs)`, `clearSkeleton()`, generated `free()`.

- [x] Write and run failing skeleton input tests for ownership, malformed data, stale input, and clear/recovery.
- [x] Implement input validation/storage and the Rust axis/orbit scene plus browser surface wrapper.
- [x] Run `cargo test -p belfast-experiment-template` and wasm target check.

### Task 2: Web host and documentation

**Files:** `experiments/template/{package.json,index.html,vite.config.ts,tsconfig.json,README.md}`, `web/{main,input,kinetic}.ts`, `web/style.css`, workspace package registration.

**Interfaces:** TypeScript calls the generated Wasm API. `createSkeletonAdapter(app)` accepts Kinetic-compatible poses without requiring Kinetic at runtime.

- [x] Add Vite startup/build commands, pointer capture/cleanup, RAF/resize/error handling, and reusable skeleton adapter.
- [x] Document running, copying, rebuilding Rust, API layout, coordinate conventions, and optional Kinetic wiring.
- [x] Build Wasm and Vite; run TypeScript checks, meaningful input tests, and Rust workspace checks.
- [x] Open the page and verify visible axes, orbit, pan, zoom, resize, and browser errors; request an independent code review and address actionable findings.

## Execution record

- Initial checkout contains unrelated changes in particle examples. This template is additive.
- Skeleton tests intentionally start with a no-op boundary to demonstrate failures before implementation.

- Rust workspace: `cargo test --workspace --all-targets` passed (208 tests).
- Template TypeScript: four tests passed, including pan/coordinate forwarding, wheel normalization, pointer cancellation/cleanup, and skeleton packing.
- Existing web suite: 34 tests passed; existing web TypeScript check passed.
- Release Wasm + TypeScript + Vite build passed. Native workspace Clippy and template-only Wasm Clippy passed with warnings denied.
- Full-dependency Wasm Clippy encounters three pre-existing `arc_with_non_send_sync` diagnostics in `crates/belfast/src/buffer.rs` and `device.rs`; these unrelated core files were not changed.
- Browser: visible axes, mouse orbit, wheel zoom, and 1280×720 → 800×600 → default resize verified; no browser warnings/errors. Pan forwarding and cancellation were checked by the input event test, not a live modified drag.
- Independent read-only review found no actionable issues. Existing unrelated particle edits were excluded.
- Local Vite preview remains available at http://localhost:5174/.
