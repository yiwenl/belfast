# Moving-camera 1080p optimization

## Goal and protocol

Add a replayable camera path, then optimize until mean FPS reaches 60 at a physical 1920 x 1080 drawing buffer, or five rounds finish. Accept/commit only candidates with repeatable gain of at least 1 FPS over their current parent. Save raw results and discarded patches. The requested benchmark instrumentation is setup, not a claimed optimization. No test suites; compile, inspect runtime, validate raw sample integrity and benchmark.

Starting renderer: `3a554df`. Default 10,000 fireflies, movement 0.27, frequency 0.28, flash 1.5 s, FOV 40 degrees, simplification enabled, seed 1831565813. Warm up 120 stationary frames at the path origin, reset seed and clock, then measure 1,200 fixed 1/60-second steps. Repeat runs and return to parent when needed; FPS aggregate is total frames / total wall time. Confirm a successful result with 3,600 frames. Never lower resolution, particle count or visual thresholds to claim improvement.

`forest-loop-v1`: phase = 2*pi*(frame/60)/20. Eye = (0.35*sin(phase), 0.06*sin(2*phase), -0.4*(1-cos(phase))). Yaw = 0.3*sin(phase), pitch = 0.08*sin(2*phase); look direction = (sin(yaw)*cos(pitch), sin(pitch), -cos(yaw)\*cos(pitch)). Every run starts at the scan origin, independent of interactive camera position. The 20-second loop repeats unchanged for longer runs. Manual camera operation is disabled until completion/cancellation, then the previous view returns. FOV remains the selected control value and is recorded.

Moving and static runs have different `cameraPath` metadata and cannot be compared by the UI. The initial experiments used schema/storage v2; final measurements use v3 with `sortCadence: "threshold-v1"`. Earlier storage keys remain untouched. Raw exports include actual camera matrices every 60 frames and on the final frame, alongside all RAF intervals and GPU samples. The asynchronous sort worker continues running during measurement; its CPU work, result handling and uploads contribute to real FPS. Worker completion frames can differ with scheduling, so matching path is not a claim of bit-identical async visibility/simulation. GPU timestamp excludes presentation/readback/CPU as before. See the R3 review correction below: v2 used an overly eager sort retry; only v3 has the normal interactive thresholds. Do not compare performance across these protocols.

## Round 1 plan

1. **Depth opacity rejection:** vertex-stage skip splats whose packed opacity cannot reach the existing 0.3 occlusion threshold. Preserve color rendering and depth threshold.
2. **Depth support bounds:** shrink the depth quad to the existing alpha >= 0.3 contour, conservatively padded; preserve depth fragment validation and image quality.
3. **Project once in compute:** cache screen-space center/ellipse/color once per visible Gaussian each changed frame, reuse in color and depth vertices. Include projection costs in GPU timing; consider memory-bandwidth costs. Only attempt if still below target after the earlier candidates.

If round 1 fails to reach target, research and document a fresh list before the next round. Stop at success or after round 5; do not label an unmet target achieved.

## Sources

- [Fast Gaussian Rasterization](https://github.com/dendenxu/fast-gaussian-rasterization): project covariance once per Gaussian rather than per quad vertex. WebGPU has no geometry shader, so compute is the candidate here.
- [Original 3DGS forward rasterizer](https://github.com/graphdeco-inria/diff-gaussian-rasterization/blob/main/cuda_rasterizer/forward.cu): preprocessing projection and alpha cutoffs provide context for avoiding repeated work without changing contributions.
- [gsplat rasterization](https://github.com/nerfstudio-project/gsplat/blob/main/docs/source/apis/rasterization.rst): projection, tiling, sorting and alpha composition are separate cost centers; this experiment retains its existing raster pipeline first.

## Measurements and decisions

### Instrumentation and baseline

Release watch builds #37/#38 passed. TypeScript initially ran before Wasm generated the new two-argument `beginBenchmark` declaration and rejected the old signature; it passed after the completed Wasm build. No test suite ran. Actual route screenshots show changing views. Two 1,200-frame baseline runs have identical complete config and all 21 sampled camera matrices, confirming deterministic path replay.

| Run        |     FPS | GPU total mean ms | GPU total P95 ms |
| ---------- | ------: | ----------------: | ---------------: |
| baseline-1 | 36.6693 |           25.0058 |          29.4257 |
| baseline-2 | 36.8019 |           24.9213 |          28.9014 |

Individual GPU pass spans can include overlap/pipeline waiting and must not be added to infer total frame duration. Use the whole-frame span for GPU comparisons. The UI saved baseline-1 in v2 storage; previous v1 results remain untouched.

### R1.1 - depth opacity rejection - DISCARD

Separate depth vertex entry skipped alpha bytes below 77 (less than the existing 0.3 depth cutoff) before covariance projection. Runs: 34.8186 / 36.8189 FPS, GPU total 26.1514 / 24.7093 ms. The faster run is only 0.0834 FPS over the two-run baseline aggregate, not a repeatable >=1 FPS improvement. Discarded the rejection; retained raw data and `patches/r1-depth-reject.patch`. Release build #39 passed and browser errors were empty. The next candidate uses a separate entry only to shrink depth support, without this rejection branch.

### R1.2 - depth support bounds - KEEP

Use the same covariance and alpha cutoff, but depth-only vertices cover radius min(3, sqrt(max(0, 2\*log(alpha/0.3))) + 0.001). Keep the original fragment test and three-sigma color footprint; the small padding conservatively covers floating-point edge uncertainty. This removes no accepted depth samples by design.

Runs: 39.3229 / 39.7899 FPS (GPU total 23.1901 / 23.0219 ms). Returned to parent `f69d14a`: 36.7716 FPS / 24.9629 ms. Candidate aggregate **39.5550 FPS**, **+2.7834 FPS** versus the fresh control; both runs individually exceed +1 FPS. All sampled camera matrices match the baseline. Builds #40/#41 passed; restored the accepted shader afterward. Keep and commit independently. Target 60 FPS not yet reached.

### R1.3 - compute projection reuse - DISCARD

Projected clip center and ellipse axes once per visible splat in a 64-thread compute pass; shared a 32-byte-per-splat storage record between color/depth vertices. Extra allocation 131,723,168 bytes for this asset; indices also bound as storage. Expanded timing to 14 queries and included projection in the forest span and whole-frame GPU span. Release build #43 and runtime passed with no errors. Same visible Gaussian math/cutoffs; initial view visually matched.

Runs: 36.4823 / 38.5379 FPS, GPU total 25.0286 / 23.7312 ms. Both slower than parent depth-bounds runs 39.3229 / 39.7899 FPS. Discard; restored shader, renderer, scene and timestamp code to `7edc93f`. Full candidate including new shader archived as `patches/r1-compute-projection.patch`. Additional memory traffic is a plausible explanation, not an isolated measured cause.

## Round 2 plan

Round 1 best is about 39.56 FPS, below 60. Refresh the bottleneck hypotheses without reducing quality:

1. Reverse only the depth instance order to front-to-back, retaining exact per-fragment depth cutoff and color order. Earlier near coverage should let depth rejection avoid far work.
2. Cover each Gaussian disk with a circumscribed triangle (3 vertices / 1 triangle) instead of a quad (4 vertices / 2 triangles). Keep analytic Gaussian support/cutoffs. Measure the tradeoff between reduced geometry work and increased empty triangle area.
3. Bypass the forest color-cache copy while moving; capture the unchanged view on the first stationary frame so static caching remains available. This removes a full-screen intermediate pass from moving frames.

Attempt in order, stop if the target is confirmed; at most five rounds total.

Round 2 research: [NVIDIA GPU Gems graphics pipeline performance](https://developer.nvidia.com/gpugems/gpugems/part-v-performance-and-practicalities/chapter-28-graphics-pipeline-performance) recommends front-to-back depth order; [MJP early-Z analysis](https://therealmjp.github.io/posts/to-earlyz-or-not-to-earlyz/) explains that discard permits early rejection on some hardware but defers depth writes. Applicability to this device must be measured. The circumscribed triangle is our geometric hypothesis, not a claimed result from those sources.

### R2.1 - front-to-back depth - DISCARD

Read the existing index buffer in reverse order in the depth vertex entry (storage binding), preserving far-to-near color order. Same min-depth and alpha test. Runs 37.9285 / 42.4139 FPS; fresh parent `7edc93f` 39.5370 FPS. Candidate aggregate **40.0460 FPS**, gain **0.5090 FPS**, below 1 FPS and not consistently faster. Discard. Build #45 passed; #46 restored parent. Archive `patches/r2-depth-order.patch`.

### R2.2 - circumscribed triangle - DISCARD

Changed both splat passes to one triangle / three vertices with local corners (-sqrt(3), -1), (sqrt(3), -1), (0, 2), scaled by the current support radius with 1.0001 conservative padding. The same fragment disk/alpha rejection remains. Runs 27.0825 / 25.9232 FPS, GPU total 34.3228 / 35.9649 ms, clearly below current parent (~39.54 FPS). Discard. Candidate patch `patches/r2-triangle.patch`; release #47 and browser runtime passed. Increased empty triangle coverage is a plausible cause; no isolated hardware-counter claim.

### R2.3 - direct moving-frame color - DISCARD

Dirty frames rendered directly to the surface with no color-cache copy. The first unchanged frame captured the cache, then resumed static reuse. Timestamp 8/9 spanned direct drawing or capture/copy as appropriate. Runs 37.9327 / 37.3686 FPS, GPU total 23.9172 / 24.3603 ms; both below parent 39.5370 FPS. Discarded; original scene restored. Release #48 and runtime passed; patch `patches/r2-direct.patch` retained.

## Round 3 plan

Current accepted renderer remains `7edc93f` (~39.56 FPS), below 60. Change the duplicated forest pass itself:

1. **Combined color and occlusion:** if `FLOAT32_BLENDABLE` is supported, write HDR color plus an R32Float minimum-depth color attachment in the same splat pass. Alpha >=0.3 contributes its original fragment depth; all other accepted color fragments contribute 1 to the min reduction. Firefly fragments compare against this same-precision depth texture. Preserve the existing separate pass on devices without the feature. This removes duplicate geometry/projection while retaining opacity support and float32 depth. Include all work in existing whole-frame/color timing.

Research and findings below determine any follow-up; never substitute reduced precision or weakened occlusion just to reach the target.

Research: the [WebGPU specification](https://www.w3.org/TR/webgpu/) defines MIN blending and the optional float32 blendability capability. [Chrome's WebGPU 132 announcement](https://developer.chrome.com/blog/new-in-webgpu-132) documents `float32-blendable` for R32Float/RG32Float/RGBA32Float attachments. Installed wgpu 30 exposes it as `FLOAT32_BLENDABLE`. The adapter feature is checked and requested only when available.

### R3.1 - combined color and occlusion - KEEP

The existing far-to-near color pass writes a second R32Float attachment with MIN blending. Alpha >=0.3 writes the original fragment depth; otherwise it writes 1. Firefly fragments load that texel and retain the original depth comparison. Float32 precision, color blending, Gaussian support, resolution, firefly count and flash settings are preserved. The separate forest depth draw is skipped on supported devices. The original depth-bounds optimization remains in the fallback. Cache invalidation and copy behavior are unchanged. This reuses the existing depth allocation (8,294,400 bytes at 1080p); no projection cache from R1.3 remains.

Initial v2 runs: **65.9943 / 64.1855 FPS**, GPU total **13.2116 / 13.4519 ms**. Reverting to the actual parent `7edc93f` returned **34.3757 FPS / 26.4218 ms**. These are valid within the original v2 stress workload, but they are not used as the final acceptance comparison after the correction below.

#### Review corrections and fresh matched measurements

Read-only review found two issues before commit:

1. The separate-pass fallback had a two-slot render attachment array ending in `None`, while its pipeline had one target. Slice to one target on fallback, two on merged. Forced the optional feature off and completed a full 1,200-frame runtime benchmark without errors.
2. `camera_frozen` also covers moving benchmarks, so a late worker result reset the sort gate on every pose mismatch and bypassed normal movement thresholds. Exact-pose retries now apply only before the measured route starts. Measurement retains the ordinary 0.02-distance / 0.5-degree sorting thresholds. Incremented benchmark schema/storage to v3 and added `sortCadence` to validation/comparison metadata so v2 results cannot silently compare against v3.

The fresh control is the candidate with `FLOAT32_BLENDABLE` deliberately not requested, exercising the original separate depth/color path with exactly the same corrected benchmark and fallback fix. The feature request was restored before all following merged runs and in the committed source.

| v3 run                                         | Frames |     FPS | GPU total mean ms | GPU total P95 ms |
| ---------------------------------------------- | -----: | ------: | ----------------: | ---------------: |
| r3-cadence-parent-1 (separate pass / fallback) |   1200 | 38.6103 |           23.8185 |          28.1149 |
| r3-cadence-merged-1                            |   1200 | 66.4481 |           13.2559 |          15.5320 |
| r3-cadence-merged-2                            |   1200 | 66.8706 |           13.1317 |          15.5976 |
| r3-cadence-merged-long                         |   3600 | 63.1902 |           13.7816 |          16.9083 |

The two matched short runs aggregate **66.6587 FPS**, a **28.0484 FPS / 72.64%** gain over the fresh control. Both individually exceed the required 1 FPS improvement. Complete configs and all 21 sampled camera matrices match the control. The longer run repeats the same route three times and averages **63.1902 FPS**; frame P95 is **25.0 ms**, so reaching average 60 does not mean every frame meets 16.67 ms. Stop after **round 3**; rounds 4/5 are not needed.

## Final verification and artifacts

- Release watch builds #52-55 passed, including forced fallback, normal-cadence fixes and final formatting. Final `pnpm --dir experiments/firefly_forest build` succeeded: release Wasm, TypeScript and Vite. Full output is in `final-build.log`. No test suites ran, as requested.
- Browser reports no errors. Physical canvas verified as 1920 x 1080 (CSS 833 x 469, DPR 2). Static mode completed 120 frames (`runs/runtime-static.json`, excluded from optimization comparisons); cancellation restored controls and FOV changed 40 -> 41 -> 40 successfully.
- Review follow-up confirmed both fixes and v3 compatibility checks, with no remaining findings in the reviewed scope.
- `runs/` preserves all 24 completed measurements (30,120 measured frames), including losing candidates, controls and the static runtime check. `summary.json` contains recomputed metrics, decisions and sample-integrity checks. `patches/` preserves the five discarded implementations; none remain in active renderer code.
- `merged-final-short.png` shows the matched UI comparison; `merged-final-long.png` shows the 3,600-frame result. The UI correctly refuses a percentage comparison when frame counts differ.
- Result applies to this browser/GPU, route, FOV and 10,000-firefly configuration with `float32-blendable`. The fallback is runnable but did not meet 60 FPS. No claim is made for 100,000 fireflies, all camera paths, other hardware or pixel-perfect cross-device equivalence. GPU pass timings may overlap; use whole-frame span for total cost.

Commits in this task: `f69d14a` adds the fixed route; `7edc93f` retains depth support bounds; the commit containing this final journal retains combined color/occlusion, review fixes and all remaining experiment evidence. No push was requested.
