# Firefly Flocking Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. Keep the existing uncommitted experiment in place.

**Goal:** Render 10,000 slowly flocking, glowing fireflies in the forest using GPU compute and locally synchronized circular blink cycles.

**Architecture:** Two particle storage buffers alternate read/write ownership. Three GPU passes clear a uniform spatial grid, insert particles into atomic linked lists, then update each particle from the surrounding 27 cells. Additive billboards use a Gaussian depth prepass for approximate forest occlusion; cache static forest depth until the camera or sorted indices change.

**Tech Stack:** Rust, Belfast/wgpu 30, WGSL, existing Wasm/Vite host.

**Spec:** User's requirements in this chat: 10,000 particles; separation/cohesion/alignment plus curl noise; shortest circular phase difference slows leaders and accelerates followers; acceleration integrates velocity, velocity integrates position; ground and ceiling bounds; compute shader simulation.

## Global Constraints

- Keep existing forest rendering and input behavior.
- No per-frame particle CPU readback; use spatial neighbor search rather than all-pairs by default.
- Default bounds: X [-5, 5], Y [-0.45, 3], Z [-12, 3], in the existing Y-up scan coordinates. The ground is a configurable horizontal simulation plane, not a mesh collision system.
- Use a soft boundary force and a hard clamp as a final bound guarantee. Keep particle bodies above the floor.
- Keep this work uncommitted with the user's existing experiment changes.

## Review Focus

- Circular phase wrapping at 0/1 and no neighbors; GPU fixture assertions.
- Coincident particles and cells with dense populations; no dropped list entries or self-neighbors.
- Zero/large dt, viewport resizing, camera movement, and initialized ping-pong state.
- Faint splats must not completely occlude the forest; firefly depth must not modify forest color blending.
- Browser WebGPU limits, shader validation, 10,000 particle performance, cleanup.

### Task 1: GPU flocking and oscillator simulation

**Files:** src/fireflies.rs, src/firefly-state.wgsl, src/firefly-update.wgsl, src/firefly-noise.wgsl, src/fireflies_tests.rs, src/lib.rs.
**Interfaces:** FireflySystem::new(&Device), update(&Device, dt, &SortCamera), simulate(&mut CommandEncoder), draw(&mut RenderPass). Parameters and initial states are private Rust PODs, configured by explicit constants/default settings.

- [x] Write tests for initialization, WGSL validation, GPU circular phase direction, separation/cohesion/alignment, zero dt, bounded stable 10,000-particle updates.
- [x] Verify tests fail before the implementation exists.
- [x] Implement spatial grid, circular coupling, analytic curl, velocity/position integration and containment.
- [x] Run Rust tests and explicit native GPU tests.

### Task 2: Glowing billboards and forest integration

**Files:** src/firefly-draw.wgsl, src/fireflies.rs, src/splat.rs, src/splat.wgsl, src/scene.rs.
**Interfaces:** SplatRenderer::draw_depth, DEPTH_FORMAT, FireflySystem interfaces from Task 1.

- [x] Verify GPU brightness changes over the cycle and depth hides a particle behind an occluder.
- [x] Add bright core/soft halo additive rendering, scene-owned depth texture, and cached splat depth pass.
- [x] Render current compute output after forest color; retain alpha blending of the original forest.
- [x] Verify resize, camera input and browser performance on the full forest asset.

### Task 3: Delivery and verification

**Files:** README.md, index.html (particle count), optional comments in web/main.ts.

- [x] Document tuning constants, phase mathematics, ground/ceiling assumptions and commands.
- [x] Run Rust/TS tests, GPU tests, release Wasm/Vite build, formatting checks.
- [x] Independently review only changed forest files; resolve actionable findings.
- [x] Refresh the existing localhost preview and capture the result.

## Verification results

- Rust suite including original Gaussian GPU rendering and full forest load: passed.
- Seven firefly tests including five native GPU fixtures: passed (phase seam, forces, 10,000-particle bounds, crowded cell traversal, glow and splat occlusion).
- Six TypeScript tests, release Wasm/Vite build, formatting: passed.
- Browser: full forest plus 10,000 fireflies rendered, camera interaction worked, no WebGPU warnings/errors; approximately 50–52 FPS in the initial view on this machine.
- Independent read-only review found no implementation defects; corrected the cross-cell fixture and added dense-cell coverage. Resize depth invalidation was inspected during review.
