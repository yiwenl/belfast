# Moving-camera 1080p optimization

## Goal and protocol

Add a replayable camera path, then optimize until mean FPS reaches 60 at a physical 1920 x 1080 drawing buffer, or five rounds finish. Accept/commit only candidates with repeatable gain of at least 1 FPS over their current parent. Save raw results and discarded patches. The requested benchmark instrumentation is setup, not a claimed optimization. No test suites; compile, inspect runtime, validate raw sample integrity and benchmark.

Starting renderer: `3a554df`. Default 10,000 fireflies, movement 0.27, frequency 0.28, flash 1.5 s, FOV 40 degrees, simplification enabled, seed 1831565813. Warm up 120 stationary frames at the path origin, reset seed and clock, then measure 1,200 fixed 1/60-second steps. Repeat runs and return to parent when needed; FPS aggregate is total frames / total wall time. Confirm a successful result with 3,600 frames. Never lower resolution, particle count or visual thresholds to claim improvement.

`forest-loop-v1`: phase = 2*pi*(frame/60)/20. Eye = (0.35*sin(phase), 0.06*sin(2*phase), -0.4*(1-cos(phase))). Yaw = 0.3*sin(phase), pitch = 0.08*sin(2*phase); look direction = (sin(yaw)*cos(pitch), sin(pitch), -cos(yaw)\*cos(pitch)). Every run starts at the scan origin, independent of interactive camera position. The 20-second loop repeats unchanged for longer runs. Manual camera operation is disabled until completion/cancellation, then the previous view returns. FOV remains the selected control value and is recorded.

Moving and static runs have different `cameraPath` metadata and cannot be compared by the UI. Benchmark schema/storage v2 keeps older v1 baselines separate. Raw exports include actual camera matrices every 60 frames and on the final frame, alongside all RAF intervals and GPU samples. The ordinary asynchronous sort worker continues running during measurement; its CPU work, result handling and uploads contribute to real FPS. Worker completion frames can differ with scheduling, so matching path is not a claim of bit-identical async visibility/simulation. GPU timestamp excludes presentation/readback/CPU as before.

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
