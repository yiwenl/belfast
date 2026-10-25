# Firefly benchmark implementation plan

Goal: manually run repeatable A/B measurements with a seed, fixed 1/60-second
simulation steps, animation-frame FPS, GPU timestamps, and a locally saved baseline.

User constraints: no automated tests; compile and confirm the page runs. Do not
optimize the Gaussian renderer in this change.

- [x] Add seeded reset and an offscreen-neighbor-search toggle.
- [x] Add optional timestamp-query support. Record each compute/render pass;
      resolve into per-frame buffer regions and map once after the run.
- [x] Add camera freeze, fixed-step rendering, GPU drain, and snapshot Wasm APIs.
- [x] Add a benchmark controller and panel: settle sorting, warm up 120 frames,
      reset state, drain uploads, measure N frames, drain/read GPU results.
- [x] Save versioned results in localStorage; compare only matching scene,
      camera, viewport, seed, frame count and settings (except the A/B toggle).
- [x] Abort on hidden page, resize, lost surface, cancellation, or GPU error;
      restore ordinary controls and timing.
- [x] Compile Rust/Wasm and TypeScript; inspect page startup and runtime errors.

Review focus: exact N frame intervals; exclude setup/drain from FPS; keep
asynchronous mapping outside the measured loop; distinguish unavailable GPU
timing from zero; reject incompatible or malformed saved baselines.

Verification: release Wasm and TypeScript compile; one short 60-frame browser
run returned 60 GPU samples and FPS statistics without console errors. No
automated tests were run. Static review found a late asynchronous GPU-error
gap; per-tick and pre-completion error checks now prevent saving a failed run.
