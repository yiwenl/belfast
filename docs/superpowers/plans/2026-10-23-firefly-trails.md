# Firefly trails implementation plan

**Goal:** Switch between existing firefly particles and solid trails covering 16 consecutive simulation frames, following ribbon02's triangular tubes.

**Architecture:** Record positions in a GPU ring buffer after simulation. Draw an indexed, instanced triangular tube per particle with opaque color, independent depth testing and the existing forest occlusion. Preserve the current sidebar work.

**Tech stack:** Existing Rust, wgpu/WGSL, wasm-bindgen and TypeScript; no new dependencies.

**Spec:** User request in this chat; reference `/Users/yi-wenlin/Development/Sketches/experiments/ribbon02`.

## Constraints and review focus

- Exactly 16 samples; one per rendered simulation frame, with no extrapolation.
- Fill history at startup, count growth and benchmark reset to avoid origin spikes or stale paths.
- Render this frame's output; wrap the ring without reversing chronology.
- Solid colors reuse the current palette without glow, additive blending or blink fading.
- Preserve forest occlusion on both merged and separate depth paths; do not write trails into cached forest depth.
- Lock the mode during benchmarks and record it in baseline compatibility.
- Continue in the active checkout so the running preview includes the already requested sidebar changes; leave unrelated files intact.

## Tasks

- [x] Add failing GPU history/solid rendering tests and WGSL validation coverage.
- [x] Implement history capture, indexed tube geometry and forest-occluded solid rendering in `src/firefly_trails.rs` and its WGSL files; integrate with `fireflies.rs` and `scene.rs`.
- [x] Expose `setFireflyTrails(bool)` through WASM; add a concise render selector in the firefly group, benchmark compatibility and README notes.
- [x] Run the experiment's Rust and frontend tests, focused GPU tests, release build and browser toggle check; inspect console and save a preview screenshot.

## Progress

- Reference inspected: ribbon02 uses three-sided instanced tubes and a circular history atlas. Use storage buffers here; no fluid advection or shadow-map feature is needed for this request.

- Verification: 14 standard Rust tests, 16 explicit GPU tests and 11 frontend tests passed. Release WASM/typecheck/Vite build and formatting checks passed.
- Browser: particle/trail toggling, 100,000-particle history growth, benchmark lock/cancel and restored controls checked; no console warnings/errors.
- Independent focused review: no actionable findings.
- Preview retains 20,000 particles, original motion settings and trail mode. Exact 16-frame history is visually short at the current 120 FPS / 0.27× motion; no extra sampling interval or stretched trajectory was added.

## 64-frame follow-up

- Increased history and tube geometry to 64 frames; updated UI labels, memory notes and benchmark version to v12 so 16-frame results remain separate.
- Updated GPU coverage to wrap the 64-frame ring and render its full history. Both targeted GPU tests, 5 standard library tests, 11 frontend tests and the release build passed.
- Browser checked at 100,000 particles with no console warnings/errors; restored 20,000 particles and trail mode for the preview. No full performance comparison was run.

## 256-frame follow-up

- Increased history to 256 frames. A new maximum-population GPU test first reproduced the 409.6 MB allocation exceeding the default 256 MiB buffer limit.
- Split history into four 64-frame buffers, each 102.4 MB at maximum capacity; the draw shader reads across chunks while ordinary recording touches only the current chunk. No device-limit increase or particle-count reduction was needed.
- Updated full-ring wrap/reset and solid-rendering GPU tests, labels, memory notes and benchmark version v13. All 3 focused GPU tests, 14 standard Rust tests, 11 frontend tests and the release build passed.
- Saved four 1,200-frame benchmark runs under `experiments/firefly_forest/benchmarks/2026-10-23-trails-256/`: 20,000 particles averaged 94.80 FPS static / 65.29 moving; 100,000 averaged 21.05 / 19.91 FPS. Browser console had no warnings/errors.
- Preview restored to 20,000 particles in trail mode. Sampling remains once per render as requested; no fixed-time sampling or additional performance optimization was introduced.
