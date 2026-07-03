# Rust wgpu First Milestone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Belfast Rust milestone: a Rust workspace with core semantic APIs, a `wgpu` runtime skeleton, and a WebAssembly facade skeleton that mirrors the existing Belfast naming.

**Architecture:** Keep the current TypeScript package untouched. Add `crates/belfast` as the Rust crate containing semantic modules plus `wgpu` runtime wrappers, and `crates/belfast-wasm` as a wasm-bindgen facade. Start with CPU-testable APIs, then compile the runtime surface.

**Tech Stack:** Rust 2021, Cargo workspace, `wgpu`, `bytemuck`, `thiserror`, `wasm-bindgen`, `wasm-bindgen-futures`, WebGPU/WGSL.

---

### Task 1: Core Semantic API

**Files:**

- Create: `Cargo.toml`
- Create: `crates/belfast/Cargo.toml`
- Create: `crates/belfast/src/lib.rs`
- Create: `crates/belfast/src/error.rs`
- Create: `crates/belfast/src/uniform_block.rs`
- Create: `crates/belfast/src/camera.rs`
- Create: `crates/belfast/src/geom.rs`
- Test: `crates/belfast/tests/uniform_block.rs`
- Test: `crates/belfast/tests/camera.rs`
- Test: `crates/belfast/tests/geom.rs`

- [x] **Step 1: Write failing tests**

Add tests for `UniformBlock` byte layout and set-by-name behavior, `PerspectiveCamera`/`OrthographicCamera` matrix output shape, and `Geom::plane` vertex/index generation.

- [x] **Step 2: Run tests to verify failure**

Run: `cargo test -p belfast --tests`
Expected: FAIL because the `belfast` crate modules and APIs are missing.

- [x] **Step 3: Implement minimal semantic APIs**

Implement runtime-schema `UniformBlock`, camera matrix math, and simple plane geometry.

- [x] **Step 4: Run tests to verify pass**

Run: `cargo test -p belfast --tests`
Expected: PASS for semantic tests.

### Task 2: wgpu Runtime Skeleton

**Files:**

- Create: `crates/belfast/src/device.rs`
- Create: `crates/belfast/src/buffer.rs`
- Create: `crates/belfast/src/mesh.rs`
- Create: `crates/belfast/src/draw.rs`
- Modify: `crates/belfast/src/lib.rs`
- Test: `crates/belfast/tests/mesh.rs`

- [x] **Step 1: Write failing mesh tests**

Add tests for vertex layout generation, automatic slot assignment, duplicate slot rejection, and index-buffer metadata.

- [x] **Step 2: Run tests to verify failure**

Run: `cargo test -p belfast mesh --tests`
Expected: FAIL because mesh/runtime APIs are missing.

- [x] **Step 3: Implement minimal runtime wrappers**

Implement cloneable `Device`, `Buffer`, `Mesh`, and `Draw` wrappers around `wgpu` with the naming needed by first examples.

- [x] **Step 4: Run tests to verify pass**

Run: `cargo test -p belfast --tests`
Expected: PASS for CPU tests and compile the `wgpu` runtime surface.

### Task 3: WebAssembly Facade Skeleton

**Files:**

- Create: `crates/belfast-wasm/Cargo.toml`
- Create: `crates/belfast-wasm/src/lib.rs`
- Create: `packages/belfast-wasm/package.json`
- Modify: `pnpm-workspace.yaml`

- [x] **Step 1: Add wasm facade exports**

Expose wasm-bindgen classes for `Device`, `Buffer`, `Mesh`, `Draw`, `UniformBlock`, `PerspectiveCamera`, and `OrthographicCamera` with JavaScript-friendly names.

- [x] **Step 2: Build check**

Run: `cargo check -p belfast-wasm --target wasm32-unknown-unknown`
Expected: PASS if wasm dependencies and target are installed.

### Task 4: First Milestone Documentation

**Files:**

- Create: `docs/features/rust-wgpu-api-parity.md`
- Modify: `docs/features/rust-wgpu-rendering-engine-prd.md`

- [x] **Step 1: Add parity matrix**

Document TypeScript, Rust native, and WebAssembly status for first-milestone exports.

- [x] **Step 2: Final verification**

Run: `cargo fmt --check`, `cargo test -p belfast --tests`, and `cargo check -p belfast-wasm --target wasm32-unknown-unknown`.
Expected: PASS, or documented blocker if a local target/toolchain is missing.
