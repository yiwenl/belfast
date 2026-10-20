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
