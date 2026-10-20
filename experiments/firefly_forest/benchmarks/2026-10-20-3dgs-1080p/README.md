# 1920 × 1080 3DGS optimization continuation

## Request / protocol

Continue the previous unfinished candidates at a fixed **1920×1080 drawing buffer**, target **60 average FPS**, up to **five rounds** if needed. Keep a candidate only when it improves mean FPS by at least **1 FPS** against its current parent; discard otherwise. Complete the two previously unattempted candidates, then reassess whether another round is needed. Preserve quality, HDR, splat data and firefly behavior. No test suites; release builds, browser checks, visual inspection and real benchmarks.

Starting commit: `4cbd6b6`, containing the accepted four-vertex renderer `d5e6c3e`. The resolution change is requested setup, not a claimed optimization. Drawing buffer is 1920×1080 independent of devicePixelRatio, and CSS preserves 16:9 with letterboxing. Pointer input remains in canvas-local CSS coordinates. Projection uses actual drawing-buffer aspect. Keep unrelated workspace modifications untouched.

New resolution and horizontal FOV change the visible splat set, so do not compare FPS to the older 1666×1976 series. Independent records live here. Seed 1831565813, fixed dt 1/60, 120 warmup frames then seed reset, 1,200 measured frames; 10,000 fireflies, movement 0.27, blink 0.28, duration 1.5 seconds, vertical FOV 40°, offscreen simplification enabled. Repeat candidate/parent checks for noise; aggregate FPS = total frames / total wall time. Export complete raw frame/GPU samples. A longer 3,600-frame run can confirm sustained performance separately from the equal-length comparisons.

## Round 1 — remaining candidates from previous list

1. **Ellipse projection with Cholesky basis (previous 1.3).** Replace the symmetric 2×2 eigensystem with a lower-triangular square root when covariance is safely positive definite. Preserve the existing eigenvalue floor with the original fallback for near-degenerate matrices. The transformed Gaussian disk has the same covariance and cutoff; evaluate performance and visual equivalence.
2. **Reuse static forest color (previous 1.4).** Cache the static color image similarly to the existing cached forest depth. Redraw on camera, projection, viewport, asset or sorted-index changes, otherwise composite the cached image before animated fireflies. Preserve HDR format and occlusion. Include capture/composite in GPU timings and disclose the stationary nature of the gain; verify orbit/FOV invalidation.

If the remaining list does not reach the target, refresh research and record the next list below. Maximum rounds: five. Do not reduce resolution, particle counts, opacity quality thresholds or geometry to claim success.

## Prior research

- [Fast Gaussian Rasterization](https://github.com/dendenxu/fast-gaussian-rasterization): avoid repeated projection and cache invariant work.
- [PlayCanvas opacity support clipping](https://raw.githubusercontent.com/playcanvas/engine/main/src/scene/shader-lib/wgsl/chunks/gsplat/vert/gsplatCommon.js): previously tried and discarded at the old resolution; do not count it as a new successful candidate.
- [gsplat large-scene guide](https://docs.gsplat.studio/main/examples/large_scale.html): radius-based quality/performance tradeoff; deferred in favor of preserving current contributions.

## Measurements / decisions

### Setup and baseline

Confirmed DOM drawing buffer 1920×1080, CSS display 833×469 (fractional 16:9 layout rounded by clientWidth/clientHeight), DPR 2 is not applied to rendering. Initial visible count 1,606,335 of 4,116,349 splats. Projection/viewport in every raw result confirms the new buffer size. Baseline runs: **56.6241**, **58.0875**, returned control **60.5367 FPS**; aggregate **58.3718 FPS**.

### Attempt 1.3 — Cholesky ellipse basis — DISCARD

Change: use lower-triangular covariance square root for safely positive definite matrices; original eigensystem remains the fallback for near-degenerate/floor-clamped matrices. Preserve three-sigma support, alpha cutoffs, color/depth shader behavior and sort order. Candidate source archived in `patches/r1-cholesky.patch` (zero-context patch, not applied).

| Run             |     FPS | GPU total ms | Forest GPU ms |
| --------------- | ------: | -----------: | ------------: |
| Baseline 1      | 56.6241 |      14.4920 |       14.2361 |
| Baseline 2      | 58.0875 |      14.3548 |       14.1328 |
| Cholesky 1      | 56.7110 |      14.2562 |       14.0340 |
| Cholesky 2      | 60.8603 |      13.8425 |       13.6415 |
| Baseline return | 60.5367 |      14.1132 |       13.9027 |

Candidate aggregate **58.7124 FPS**, gain against all three baseline controls **0.3406 FPS**. Even its faster run is only **0.3236 FPS** above the fresh parent control. Not a reproducible >=1 FPS improvement: **discarded**. Browser had no errors, release build #31 passed; #32 restored original shader. Screenshot and both raw runs retained. The requested fixed-resolution setup remains, independent of this discarded optimization.

### Attempt 1.4 — static forest color cache — KEEP

Change: introduce `ForestColor` and a fullscreen `textureLoad` copy. The sorted Gaussian pass renders into a full-resolution texture in the **same format as the surface**. Reuse it together with existing forest depth when camera/view/projection, viewport, PLY asset and sorted-index contents are unchanged. No Gaussian count, cutoff, color, resolution or particle simulation reduction. Firefly compute and additive glow still execute every frame. The cache never contains fireflies.

Invalidation: camera/projection differences, viewport allocation, asset load and successful sorted-index replacement all mark the forest dirty. A dirty frame captures color/depth before compositing; clean frames only composite the cache. GPU forest timing starts before capture on dirty frames (before copy on clean frames) and finishes after copy, plus the existing optional depth span. GPU whole-frame timing still spans compute through glow; no new cost is omitted from the timestamps.

| Run                     |      FPS | GPU total ms | Forest GPU ms | Use                        |
| ----------------------- | -------: | -----------: | ------------: | -------------------------- |
| Cache exploratory       | 119.9976 |       1.0822 |        0.5224 | Excluded: different camera |
| Cache matched 1         | 119.2061 |       1.2922 |        0.5137 | Acceptance                 |
| Cache matched 2         | 119.8957 |       1.0254 |        0.4258 | Acceptance                 |
| Parent return           |  59.5067 |      13.5345 |       13.3424 | Fresh control              |
| Cache 3,600 frames      | 119.8498 |       1.0999 |        0.3807 | Sustained confirmation     |
| Cache after final build | 119.7007 |       1.0663 |        0.4072 | Acceptance / final UI      |

The first exploratory cache run had a non-identity view matrix. The existing benchmark comparison correctly rejected it. It is retained as raw evidence but excluded from the optimization decision. Reloading restored the baseline camera. The three matched 1,200-frame cache runs have **identical complete configuration** to the parent control; combined FPS is **119.6001**, versus **59.5067** for the fresh parent return (**+60.0934 FPS**). Parent source was restored from `00add7e`, rebuilt (#34), measured, then the cache candidate restored (#35). The independent 3,600-frame confirmation reached **119.8498 FPS**, with **1.0999 ms** mean GPU total. **Keep**: the improvement comfortably exceeds 1 FPS.

### Runtime / build evidence and limitations

- Inspected HDR forest and animated glow at FOV 40°, changed FOV to 55° and back, and dragged the camera. Forest image and sorted visible count changed appropriately; the forest does not remain frozen at the original view. Saved `color-cache-fov55.png` and `color-cache-orbit.png`. Browser console error list was empty after interactions and after final build.
- `pnpm --dir experiments/firefly_forest build` completed with release Wasm, TypeScript `--noEmit`, and Vite production build. `build.log` records success. An earlier overlapping watch/full build reached Rust success but failed in wasm-pack tool-install metadata parsing (`invalid type: sequence, expected a string`). After the watcher completed, the identical full build succeeded without source/config changes. Concurrency is a likely cause, not conclusively established; the original failure log is retained as `build-first-attempt.log`. Avoid running both builds concurrently.
- Final formatting was followed by a same-config 1,200-frame runtime benchmark and `final.png`. No test suite was run, as requested. Raw counts, frame sums, FPS and matching metadata were checked when generating `summary.json`.
- This is a **stationary-camera optimization**, matching the existing benchmark. Moving-camera frames still render the forest and add a full-screen copy, so the stationary result does **not** establish 60 FPS while continuously orbiting. No moving-camera A/B performance result is claimed. Reload/first capture and sorting are outside the warmup-based measurement, as before.
- The RGBA16Float color texture uses **16,588,800 bytes (15.82 MiB)** at 1080p. Depth memory was already allocated. HDR values are preserved; the copy introduces no tone mapping or filtering. Visual inspection shows no obvious regression; no pixel-identical automated comparison was performed.
- The benchmark uses **10,000 fireflies**. Results do not establish performance at 100,000. The display appears limited near 120 Hz, so GPU timings are the useful measure of remaining headroom. Individual pass timestamps may include GPU overlap/pipeline waits and need not sum to total.

## Final decision

The two remaining candidates are completed in **round 1**: Cholesky discarded; static color reuse retained. The fixed **1920×1080**, 10,000-firefly, stationary-camera benchmark exceeds the **60 FPS** target, including the 3,600-frame confirmation. Stop here; rounds 2–5 are unnecessary under the requested stop condition. No resolution or quality reduction was used to reach the target.

This continuation contains **11 raw runs / 15,600 measured frames**, including the explicitly excluded camera-mismatch run. `summary.json` records aggregate calculations and decisions; every JSON in `runs/` contains all per-frame RAF intervals and GPU samples. Fixed-resolution setup was committed as `00add7e`; the accepted cache and this evidence are committed together separately. Earlier portrait-resolution results remain in the previous journal and are not used for the 1080p comparison.

Read-only code review found no blocking issues in invalidation, HDR/sRGB copy, timestamp indices or fixed-resolution/pointer handling. It independently recomputed the first 10 raw reports (the final post-build run was added afterward and checked during summary generation). Continuous-motion FPS, hardware HDR headroom, SDR runtime, and pixel-identical image equivalence remain outside the evidence; no claim is made for them. Unrelated workspace changes were excluded.
