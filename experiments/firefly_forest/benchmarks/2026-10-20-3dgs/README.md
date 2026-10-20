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

### Attempt 1.2 — opacity-bounded support — DISCARD

Parent: `d5e6c3e`. The vertex shader unpacks alpha earlier and limits the quad's Gaussian-coordinate extent to `min(3, sqrt(max(0, 2 * log(255 * alpha))) + 0.0001)`. Padding is conservative for floating-point rounding; fragment thresholds and depth coverage remain unchanged. Hypothesis: avoid rasterizing tails already rejected below alpha 1/255. Possible cost: four extra logarithm/square-root calculations per splat. Evidence patch: [`patches/r1-opacity-support.patch`](patches/r1-opacity-support.patch), **not applied** to the renderer.

| Run                  | Mean FPS | Frame P95 ms | GPU total ms | Forest GPU ms |
| -------------------- | -------: | -----------: | -----------: | ------------: |
| Support 1            |  63.2558 |        19.90 |      12.7880 |       12.6020 |
| Support 2            |  63.9475 |        20.50 |      12.9697 |       12.7588 |
| Strip return/control |  63.4937 |        18.00 |      13.0187 |       12.8173 |

Support aggregate: **63.5997 FPS**. Gain versus fresh unchanged strip control: **0.1060 FPS**, below 1. The older strip aggregate was slower, illustrating why rechecking the parent matters. GPU savings are small and do not meet the user's FPS criterion. Release build #29 loaded with no browser errors; #30 restores the unchanged accepted shader. Decision: **discard**, no optimization commit for this patch. Raw runs are retained for audit.

### Sustained confirmation and stop decision

Accepted source remains `d5e6c3e`. Three 1,200-frame strip runs combine to **60.6207 FPS**, versus **48.0582 FPS** for three original runs: **+12.5625 FPS / +26.14%**. A separate **3,600-frame** run at the same seed, camera, resolution and other controls confirms **62.5536 FPS** over **57.5506 seconds**. GPU total mean **13.0833 ms**, P95 **14.2868 ms**; forest mean **12.8698 ms**. Wall-clock frame P95 is **21.8 ms**: the target is an average, not every frame staying below 16.67 ms.

The 3,600-frame run is sustained confirmation, **not** used for a direct percentage against 1,200-frame runs because it covers a longer simulation interval. The UI correctly marks its frame count as incompatible with the saved 1,200-frame baseline. All records have identical other config fields, complete interval/sample counts, and recomputable FPS.

Camera verification: changed FOV 40→55→40; visible splat count updates, then orbit changes the view and sorting; forest and firefly occlusion remain present, no browser/GPU error. Screenshots are included. The page is restored to the default view afterward. This does not claim 60 FPS during continuous orbit/sorting.

**Stop in round 1:** average-60 target reached without changing resolution, scene count, cutoff, HDR or firefly parameters. Candidates 1.3 (ellipse basis) and 1.4 (projection/color reuse) are **not attempted**, not measured failures. No new round needed, and no claim of five completed rounds. They remain future options if the target expands to continuous movement, wider FOV, a larger viewport, or 100,000 fireflies.

## Reproducing and interpreting the results

1. Use the same browser/display and canvas size recorded in the JSON (833×988 CSS pixels, DPR 2, 1666×1976 output), default view, 10,000 fireflies and recorded controls.
2. Run the default 1,200-frame benchmark; download the full JSON using the new link. The original 1,200-frame baseline remains saved in the browser.
3. Original renderer is available at `247c4b8`, accepted renderer at `d5e6c3e`. Benchmark export support is measurement-only and lives in the accepted commit; do not change the timed draw loop when comparing.
4. Compare total-frame/total-time FPS and GPU total/forest times. Use fresh parent controls because GPU scheduling and other local workload produced visible run-to-run variation.
5. Source PLY remains local and ignored. No tests were executed; release Wasm compilation, TypeScript checking, actual UI runs and static review provide the stated verification.

## Final validation / source state

- `pnpm --dir experiments/firefly_forest build` passed (release Rust/Wasm, TypeScript, Vite production bundle). Build log kept as `build.log`.
- Fresh read-only reviewer found no actionable source or measurement issue. Independently recomputed all nine raw runs including component means/P95s and checked discarded source equals the accepted commit.
- Nine runs total **13,200 measured frames**, each preceded by the documented warmup/reset. [Machine-readable decision summary](summary.json).
- Four-vertex implementation committed as `d5e6c3e`. This final journal update records the failed attempt and sustained confirmation; it introduces no further renderer optimization.
- `.ply` assets remain ignored and all unrelated pre-existing workspace changes remain outside these commits. No push performed.
