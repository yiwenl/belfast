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

## Drag-transition follow-up

- Added default **拖曳軌跡** mode while retaining explicit particle and 256-frame trail modes. Pressing starts visible history at zero and fades particles out over 0.25 seconds; release retracts the tail along the live curve and restores particles over 0.65 seconds. Regrabbing preserves the current transition.
- Visible history age avoids rewriting all history buffers while idle. Fractional tail sampling keeps retraction continuous, and recording continues as the particle head moves.
- Added a depth prepass only during fades so overlapping solid trails blend independently of instance order. Corrected timestamp ownership across depth, trail color and particle passes.
- Verification: 17 standard Rust tests, 17 GPU tests and 11 frontend tests passed. The extended solid-trail GPU test reproduced the overlap-order bug before its fix and passed afterward. Release WASM/typecheck/Vite build and formatting checks passed.
- Browser verified actual camera drag, hold/release and return to glowing particles. A 60-frame moving Drag run completed with 60 GPU timing samples and no console warnings/errors after reproducing and fixing the empty-timestamp-descriptor failure. No full performance comparison was run.
- Focused independent review findings resolved. Preview remains at 20,000 particles in Drag mode; benchmark frame count restored to 1,200.

## Drag-speed follow-up

- Reused the transition's smooth curve to raise motion from the configured rate to 1× over 0.25 seconds and return over 0.65 seconds. Regrabbing continues from the current rate; slider values, blinking and explicit particle/trail modes retain their behavior.
- Both motion integration and the wandering-noise clock use the effective rate. Benchmark version/key advanced to v15 because moving Drag runs now accelerate.
- A GPU displacement test failed before the change and passed afterward, covering gradual acceleration/deceleration, regrabbing, zero speed, preserved settings and independent blinking. All 18 firefly GPU tests, 17 standard Rust tests, 11 frontend tests, release build and formatting checks passed. The watcher test hit sandbox EMFILE initially and passed with normal file-watcher access.
- Browser drag/release showed longer trails followed by particles at the original 0.27× setting, with no console warnings/errors. Focused review found no issues; no full performance benchmark was run.

## Release-flash follow-up

- Stored the current flash intensity in history's unused fourth component without increasing GPU allocation. The whole trail samples its live head's flash; the existing particle fade smoothly blends between solid trail brightness and that envelope during release/regrab.
- Kept ordinary alpha blending, with no halo or added glow. Fully held and manual trails retain solid colors. Advanced benchmark version/key to v16.
- The GPU trail-render test failed before the change and passed afterward when checking live flash values 0→0.5→1→0 during retraction. Both history GPU tests, 17 standard Rust tests, 11 frontend tests and the release build passed. Browser hold/release completed without console warnings/errors; focused review found no issues. No full performance benchmark was run.

## Shared drag-brightness follow-up

- Particles and trails now share `boosted_flash`: the existing transition blends each natural flash toward full brightness over 0.25 seconds while held, then back to its live flash over 0.65 seconds on release. Simulation phases and flash ages continue unchanged, preserving each firefly's own cycle.
- Retained the original particle/trail opacity crossfade, smooth motion boost and live-head retraction. Fully held trails remain opaque with one draw pass; transition overlap keeps its depth prepass. No additional GPU allocation or control is needed. Benchmark version/key is v18.
- GPU tests first reproduced dark particles staying dark and held trails failing to reach full brightness; both pass with the shared boost. All 18 firefly GPU tests, 17 standard Rust tests, 11 frontend tests, release build and formatting checks passed. Browser hold/release showed the full illuminated field and return to individual flashes, with no console warnings/errors. Focused review found no code issues; no full performance comparison was run.

## Flash-recovery control follow-up

- Added **螢火蟲 → 閃爍恢復**, 0.1–5 seconds, default 0.65. Brightness recovery has its own timer and shared camera uniform; trail retraction, opacity and movement slowdown retain their original timing. Changing the duration during recovery or regrabbing continues from the current brightness, while natural flash cycles keep running.
- Added WASM validation and benchmark configuration/compatibility coverage; benchmark version/key is v19. No new history storage or dependencies.
- Verification: 19 standard Rust tests, all 18 firefly GPU tests, 11 frontend tests and the release WASM/typecheck/Vite build passed. A focused review found a short-recovery regrab could skip the transparent-trail depth prepass; the overlap-order GPU test reproduced it, and passed after including the independent brightness boost in the prepass condition. Formatting and diff checks passed.
- The frontend watcher test timed out once and passed on rerun; the sandboxed wasm-pack invocation failed during helper installation, while the normal-access release build succeeded. No unrelated watcher or build configuration was changed.
- Browser verified the new slider, a 2-second hold/release setting and return to the default 0.65 seconds with no console warnings/errors. No full performance comparison was run.

## Particle-glow toggle follow-up

- Added **螢火蟲 → 粒子 Glow**, enabled by default. An existing camera uniform component controls only the particle halo; the core, flash cycles, trails and forest lighting keep their behavior. Toggling does not reset particles. Benchmark v20 records and locks the setting and rejects mismatched baselines.
- The focused GPU test verifies a visible core without a halo, dark phases and forest occlusion; its close-up assertion resolves the narrow core on the 64px test target. It passed along with 19 standard Rust tests, 11 frontend tests, the release build and formatting checks. The existing watcher test timed out once during concurrent compilation and passed on rerun.
- Browser checked both toggle states and drag/release with glow off; no console warnings/errors. No full performance comparison was run.
