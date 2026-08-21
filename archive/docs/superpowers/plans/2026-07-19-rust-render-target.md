# Rust Render Target Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add render-to-texture support to the Rust Belfast crate.

**Architecture:** Add a focused `RenderTarget` wrapper around color/depth `wgpu::Texture` resources. Keep the first version close to the TypeScript Belfast `RenderTarget` surface: create, resize, color/depth views, sampler, and render-pass descriptor helpers.

**Tech Stack:** Rust 2021, `wgpu 0.20`, Belfast Rust crate tests.

## Global Constraints

- Implement only the Rust version.
- Do not add `CopyHelper` yet.
- Keep color textures sampleable with `RENDER_ATTACHMENT | TEXTURE_BINDING`.
- Keep tests focused on public behavior and metadata.

---

### Task 1: Render Target Core

**Files:**

- Create: `crates/belfast/src/render_target.rs`
- Modify: `crates/belfast/src/lib.rs`
- Modify: `crates/belfast/src/error.rs`
- Test: `crates/belfast/tests/render_target.rs`
- Modify: `docs/rust/rust-wgpu-api-parity.md`

**Interfaces:**

- Consumes: `Device`, `BelfastResult`, `BelfastError`.
- Produces: `RenderTarget`, `RenderTargetOptions`, `RenderPassOptions`, `RenderPassTarget`.

- [x] **Step 1: Write failing tests**

Tests cover dimension clamping, default options, depth metadata, resize behavior, and render pass target helper shape.

- [x] **Step 2: Run tests to verify failure**

Run: `cargo test -p belfast render_target --tests`
Expected: FAIL because `RenderTarget` does not exist yet.

- [x] **Step 3: Implement minimal code**

Add `RenderTarget` resource creation and metadata helpers.

- [x] **Step 4: Run tests to verify pass**

Run: `cargo test -p belfast render_target --tests`
Expected: PASS.

- [x] **Step 5: Verify workspace**

Run: `cargo fmt --check`, `cargo test -p belfast --tests`, `cargo check -p belfast-wasm --target wasm32-unknown-unknown`.
Expected: PASS.
