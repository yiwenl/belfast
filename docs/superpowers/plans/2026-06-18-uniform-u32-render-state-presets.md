# Uniform u32 And Render State Presets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `UniformBlock` `u32` support and render-state preset helpers while preserving existing constructors.

**Architecture:** `UniformBlock` stores one `ArrayBuffer` with typed views for float and unsigned integer writes. Render-state presets are pure helper functions that return option fragments for `Draw` and `DepthDraw`.

**Tech Stack:** TypeScript, WebGPU types, Vite library build, Node built-in `node:test`.

---

### Task 1: Add Regression Tests

**Files:**

- Create: `tests/belfast-api.test.mjs`

- [ ] **Step 1: Write failing tests**

Create `tests/belfast-api.test.mjs` with tests that import `UniformBlock`, `opaqueTriangles`, and `depthOnlyTriangles` from `../dist/belfast.js`.

- [ ] **Step 2: Run tests to verify failure**

Run `pnpm build` and `node --test tests/belfast-api.test.mjs`. Expected: failure because `u32` is unsupported and render-state helpers are not exported.

### Task 2: Implement UniformBlock u32

**Files:**

- Modify: `packages/belfast/src/core/UniformBlock.ts`
- Modify: `docs/api/UniformBlock.md`

- [ ] **Step 1: Add `u32` metadata and shared backing buffer**

Extend `UniformFieldType`, track field offsets in 32-bit slots, and add both float and uint views over the same buffer.

- [ ] **Step 2: Add integer validation and write path**

Make `set("field", number)` write `u32` values through the unsigned view and reject invalid values.

- [ ] **Step 3: Document `u32`**

Update supported field types and notes.

### Task 3: Implement Render-State Presets

**Files:**

- Create: `packages/belfast/src/helper/renderState.ts`
- Modify: `packages/belfast/src/index.ts`
- Modify: `docs/api/Draw.md`
- Modify: `docs/api/DepthDraw.md`

- [ ] **Step 1: Add preset helpers**

Create `opaqueTriangles(options)` for `DrawOptions` fragments and `depthOnlyTriangles(options)` for `DepthDrawOptions` fragments.

- [ ] **Step 2: Export helpers and types**

Export helper functions and option types from `packages/belfast/src/index.ts`.

- [ ] **Step 3: Document usage**

Add examples showing helpers spread into existing constructors.

### Task 4: Verify

**Files:**

- Run verification only.

- [ ] **Step 1: Build**

Run `pnpm build`. Expected: success.

- [ ] **Step 2: Run API tests**

Run `node --test tests/belfast-api.test.mjs`. Expected: all tests pass.

- [ ] **Step 3: Typecheck**

Run `pnpm typecheck`. Expected: success.
