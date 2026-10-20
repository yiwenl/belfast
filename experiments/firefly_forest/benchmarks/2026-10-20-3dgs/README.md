# 3DGS optimization journal — 2026-10-20

## Contract and measurement protocol

User request: inspect/search, list candidates, try one at a time, benchmark and keep/commit only improvements of **at least 1 mean FPS**; otherwise discard. Reconsider after each exhausted list, at most **five rounds**. Target: **60 mean FPS**. Stop early when the target is confirmed. Preserve output resolution, HDR, scene data, alpha thresholds and firefly behavior. Existing unrelated work stays untouched. No test suites; release builds, browser error checks, visual inspection and real browser benchmarks only.

Baseline code: `247c4b8` (firefly experiment), on branch `particles`.

Protocol: default camera, 40° FOV, seed 1831565813, 10,000 fireflies, movement 0.27, frequency 0.28, duration 1.5 seconds, view simplification enabled. 120 warmup frames, reset seed and GPU state, then 1,200 measured fixed 1/60-second steps. Keep browser/canvas dimensions, DPR, output mode and controls identical. Record frame intervals and GPU timestamps in `runs/*.json`. Compare aggregate FPS as total measured frames / total elapsed time, never the average of instantaneous FPS. Repeat baseline and candidate; near-threshold/noisy results require paired rechecks. Compilation, exports, screenshots and tool activity happen outside measured frames.

The existing benchmark locks the camera: it measures stationary rendering, not worker sorting while orbiting. Camera movement must still work and cache invalidation must be correct. Results only claim performance for the recorded configuration, not 100,000 fireflies or every camera/viewport. GPU pass timings may overlap on this architecture; use whole-frame span as the total. The JSON exporter serializes only after timing ends.

## Renderer inspection

- 4,116,349 active Gaussians; initial view submits about 1.36 million instances.
- Six vertices per instance each load textures, project covariance, solve the eigensystem, and construct the same ellipse.
- Sorted indices only change with camera motion; static attributes already live on GPU.
- Forest depth is cached by camera/projection/size/sort; color is redrawn every frame.
- Fragment shader rejects 3-sigma tails and alpha below 1/255 after rasterization.
- No resolution or geometry quality reduction is authorized by this plan.

## Research

- [fast-gaussian-rasterization](https://github.com/dendenxu/fast-gaussian-rasterization): reduce repeated projection work, cache persistent computations; small triangles can limit hardware rasterization. CUDA/OpenGL implementation is not directly portable to WebGPU.
- [PlayCanvas opacity-aware quad clipping](https://raw.githubusercontent.com/playcanvas/engine/main/src/scene/shader-lib/wgsl/chunks/gsplat/vert/gsplatCommon.js): bound rasterized support by the alpha cutoff.
- [FlashGS, CVPR 2025](https://openaccess.thecvf.com/content/CVPR2025/papers/Feng_FlashGS_Efficient_3D_Gaussian_Splatting_for_Large-scale_and_High-resolution_Rendering_CVPR_2025_paper.pdf): avoid ineffective rasterization work through more accurate intersection/culling.
- [gsplat large-scene guide](https://docs.gsplat.studio/main/examples/large_scale.html): projected-radius clipping can improve performance, but removes contributions; defer because quality-preserving changes come first.

## Round 1 — initial candidate list

1. **Four-vertex triangle strip.** Replace six duplicate vertices with four; preserve the two triangles, winding-independent rasterization, ellipse and alpha.
2. **Opacity-bounded support.** Replace radius 3 with `min(3, sqrt(2 log(255 alpha)))`, conservative padding if required; same fragment cutoff.
3. **Cheaper ellipse basis.** Evaluate a Cholesky factor or conic formulation instead of repeated eigendecomposition; preserve the covariance and visible support.
4. **Reuse invariant projection/color.** If still needed, evaluate compute projection on camera changes or cache stationary forest color, with camera, sort, viewport and asset invalidation. Measure normal camera interaction separately from the stationary win.

Only one candidate is applied at a time, against the most recent accepted code. Keep discarded patches under `patches/` as evidence, never active renderer code. If the target remains unmet after the list, record a fresh search and the next evidence-based list here; total rounds cannot exceed five.

## Runs and decisions

### Attempt 1.1 — four-vertex triangle strip — KEEP

Changes: `src/splat.rs` uses TriangleStrip for color and occlusion pipelines and draws four vertices; `src/splat.wgsl` emits (-1,-1), (1,-1), (-1,1), (1,1). The quad diagonal changes, but all Gaussian/color varyings are affine with the same clip W, and culling is disabled. Both triangles cover the same ellipse; alpha cutoff, data, viewport and simulation are unchanged.

| Run                     | Mean FPS | Frame P95 ms | GPU total ms | Forest GPU ms |
| ----------------------- | -------: | -----------: | -----------: | ------------: |
| Original 1              |  48.9814 |        25.30 |      18.4432 |       18.1913 |
| Original 2              |  43.5425 |        59.20 |      20.2639 |       19.8061 |
| Strip 1                 |  55.9795 |        25.40 |      14.2266 |       13.8464 |
| Strip 2                 |  62.9928 |        23.00 |      13.2642 |       13.0432 |
| Original return/control |  52.5146 |     see JSON |      17.3916 |       17.1811 |

Aggregate original: **48.0582 FPS** (3,600 frames). Strip: **59.2795 FPS** (2,400 frames), gain **11.2213 FPS**. Even the slower strip run exceeds the fastest original control by **3.4650 FPS**. Baseline variation is material, so raw runs are retained without cherry-picking. The original-return run after both candidate runs confirms the GPU reduction persists across time.

Validation: release Wasm build #26 succeeds; browser GPU/console error list empty; baseline and candidate screenshots preserve forest shape, brightness and occlusion. No test suites run. #27 rebuilt original for the return control. Restore the accepted strip before continuing. Screenshot firefly positions differ because screenshots were taken after the measured runs, not at identical simulation frames.

Measurement support added: downloadable full JSON with config, 1,200 RAF intervals, 1,200 four-component GPU samples, and summary statistics. Serialization happens after timing. This supports the requested audit and does not change timed rendering.

Decision: keep and commit. Aggregate result is still below 60, continue to opacity-bounded support.
