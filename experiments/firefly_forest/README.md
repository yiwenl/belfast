# Firefly forest

Renders `public/forest.ply` with the Rust/WebGPU 3D Gaussian splatting pipeline from `experiments/particles`. The forest keeps its original PLY colors. The camera starts at the scan origin, looking along negative Z after converting PLY coordinates to Y-up.

## Render resolution and static forest cache

The drawing buffer is fixed at **1920 × 1080**, independent of device pixel ratio. CSS fits the canvas to the window at 16:9 without stretching it; pointer input remains relative to the displayed canvas.

The static forest color and occlusion depth are cached together. Camera, FOV, viewport, asset or sorted-index changes invalidate both; otherwise a full-resolution texture copy precedes the animated firefly pass. The cache uses the selected surface format, preserving HDR values and the existing forest blending. At 1080p, the RGBA16Float color texture adds about **15.82 MiB** of GPU memory.

This optimization benefits a stationary camera. While the camera moves, the forest still renders every changed frame and also incurs the copy pass; the fixed-camera benchmark does not establish moving-camera performance. Firefly simulation and glow continue every frame.

## HDR output

The renderer requests HDR through Belfast's surface selection helper. Supported browsers use an `rgba16float` canvas with `ExtendedSrgb` (`toneMapping.mode: "extended"`), preserving additive firefly emission above SDR white instead of clamping it to `1.0`. The firefly shader's peak multiplier is **3.0** for HDR, compared with **1.8** for the SDR fallback. The existing flash envelope and timings are unchanged.

The browser canvas expects encoded extended sRGB, so the forest retains its original display RGB and existing blending. The color-cache copy applies no tone mapping or exposure adjustment. The overlay reports **HDR 輸出** or **SDR 輸出** from the selected surface mode; actual highlight headroom depends on the display and OS settings. Browsers without fp16 canvas support retain SDR output.

## Manual benchmark

Open the **Benchmark** panel to select a seed (`1–4294967295`, default
`1831565813`) and measurement length (`1–10000` frames, default `1200`).
The default **固定移動路徑** option runs `forest-loop-v1`, a 20-second loop
combining translation, yaw and pitch. Each camera pose uses the fixed simulation
frame index, so slower rendering does not change the route. The camera starts
at the scan origin; FOV remains the chosen control value. Disable this option
to benchmark the current static view. Completion or cancellation restores the
interactive view. Path version and sorting cadence are part of baseline
compatibility; v3 baselines are stored separately from earlier v1/v2 results.
Moving measurements use the same sort thresholds as interactive viewing;
only stationary setup forces an exact-pose sort.
The **視角外簡化** checkbox enables or disables offscreen neighbor-search
simplification both in ordinary viewing and in the next benchmark.

1. Set the particle controls and camera, choose a seed and frame count, then
   press **開始量測**.
2. The camera finishes orbital damping and manual input is locked. Moving runs
   first reset to the route origin. The renderer waits for
   the exact camera's Gaussian sort, warms up for 120 frames, resets both
   particle buffers and noise clocks to the seed, and drains the initial uploads.
3. Each measured frame advances the simulation by exactly `1/60` second.
   Wall-clock RAF intervals measure real performance; setup, warmup and final
   GPU readback are excluded. The first N simulation frames are measured.
4. Press **儲存為基準**, change **視角外簡化**, and run again. The baseline
   survives reloads in this origin's localStorage; it can be replaced or cleared.
5. **下載本次完整結果 JSON** exports the configuration, summary statistics,
   every measured frame interval, GPU samples and camera matrices sampled every
   60 frames plus the last frame, after timing finishes.

The [2026-10-20 3DGS optimization journal](benchmarks/2026-10-20-3dgs/README.md)
records candidate changes, retained/discarded decisions and raw benchmarks. The
[1080p continuation](benchmarks/2026-10-20-3dgs-1080p/README.md) records the new
resolution baseline, remaining candidates and accepted static color cache.

Results include average FPS (`N × 1000 / total milliseconds`), mean and P95 frame
time, and mean GPU times for the whole frame, all three firefly compute passes,
forest color/depth rendering including cache rebuild/copy, and firefly glow rendering. GPU whole-frame time
also reports P95. It spans the first compute timestamp through the last glow
timestamp, including inter-pass gaps but excluding presentation, timestamp
resolve/readback and CPU work. Pass timings use `timestamp-query` when the
adapter supports it. A 12-entry query set resolves into a separate 256-byte
region per frame, with one asynchronous map after the entire run; there are no
per-frame CPU waits or readbacks. Unsupported devices report GPU timing as
unavailable and still measure FPS. GPU architectures can overlap pass execution
or include pipeline waits in pass timestamps, so the individual pass durations
need not sum to the whole-frame span.

The panel compares results only when seed, frame count, step size, warmup,
particle parameters, camera, viewport, DPR, GPU/output mode, browser and scene
identifier match. The simplification flag is deliberately excluded from that
match so it can be the A/B variable. Changing visibility or viewport dimensions,
cancellation, or a failed frame invalidates the active run; completed results
and the saved baseline remain available.

Moving runs rebuild forest color/depth as the camera changes and keep the normal
asynchronous sort worker active, including result handling/uploads in real FPS.
Worker completion frames may vary with scheduling. Static runs retain cached
forest color/depth. Keep the page visible
and use identical display settings. FPS may reach the display refresh limit,
so GPU times are useful even when FPS stays unchanged. Atomic grid insertion
can vary neighbor accumulation order, and changing simplification changes
particle trajectories: the seed reproduces initial conditions, not a guarantee
of bit-identical simulation. Repeat longer runs when comparing performance.

The [moving-camera journal](benchmarks/2026-10-20-3dgs-moving/README.md) records
the exact route, matched baselines, optimization attempts and complete raw data.

## Fireflies

The **粒子數量** (particle count) slider selects **10,000–100,000 fireflies**, in steps of 1,000, defaulting to 10,000. Both particle buffers and grid links are allocated for 100,000 particles at startup; changing the active count updates compute dispatches, neighbor lookup bounds and draw instances without reallocating buffers or resetting active particles. Inactive particles are excluded from simulation and rendering.

Fireflies run entirely on GPU compute. Each frame clears a spatial grid and inserts every active particle into an atomic linked list. Particles within the camera frustum and its guard band search the 27 adjacent cells for neighbors. The lists have one entry per particle, so crowded cells do not silently drop neighbors. Particle state alternates between two storage buffers; rendering uses the latest output without CPU readback.

The compute shader skips neighbor searches, flocking forces and phase synchronization outside the expanded frustum. The guard band is one neighbor radius plus 0.1 scene units for the glow billboard (**1.0 unit** with the default settings), measured against normalized world-space planes and updated with the camera every frame, including FOV changes. Offscreen particles remain in the grid so visible particles can still interact with them. They continue moving with curl noise, drag and boundary constraints, and advance their own randomized cycles and fixed-duration flashes; neither state nor time freezes when leaving the view.

Nearby fireflies separate, more distant neighbors attract, and their velocities align. Analytic curl noise adds wandering, with an individual sampling offset and animation rate so adjacent fireflies do not follow identical paths. This reuses one curl evaluation per particle. Motion integrates acceleration into velocity, then velocity into position using small time steps, drag and a speed limit.

Each firefly has a continuous phase in `[0, 1)` and its own intrinsic frequency between 0.16 and 0.28 cycles per second. Neighbor differences use `fract(neighborPhase - phase + 0.5) - 0.5`, the signed shortest distance around the cycle. A positive average speeds the oscillator up; a negative average slows it down. Weaker coupling preserves individual timing alongside local synchronization. Each particle also has smooth random frequency drift, up to ±0.065 cycles per second, with an individual offset and rate. The phase never jumps to match a neighbor.

Each cycle wrap triggers a separate flash, with a smooth rise and fall and no glow between flashes. The **閃爍時長** (flash duration) slider selects **0.1–2 seconds**, in 0.05-second steps, with a **1.5-second default**. The GPU captures the selected duration at the start of each flash and tracks its age in unscaled seconds, including the fraction of the triggering frame after the cycle wraps. Duration changes apply to the next flash, so active pulses do not jump and expired pulses stay dark. Frequency, neighbor synchronization and jitter only affect the interval between flashes; even a frequency change during a flash leaves its duration unchanged.

The on-screen **移動速度** (movement speed) and **閃爍頻率** (blink frequency) sliders independently scale motion and oscillation from `0` to `1`, defaulting to **0.27×** and **0.28×**, respectively. `1` preserves the original speed; `0` pauses motion or prevents new flashes without resetting the cycle. An active flash finishes normally even at frequency `0`. Noise animation clocks use the square root of their respective slider rates, so wandering and frequency drift keep evolving at low settings without speeding up motion or cycle integration. Zero still freezes the corresponding noise clock. Changes take effect immediately without reloading the forest.

Tune `Settings::default()` in `src/fireflies.rs`:

| Parameter                    | Default                                          |
| ---------------------------- | ------------------------------------------------ |
| Population                   | `10,000` default, `100,000` maximum              |
| X / Z bounds                 | `[-10, 10]` / `[-24, 6]`                         |
| Ground / ceiling Y           | `-0.45` / `3.0`                                  |
| Neighbor / separation radius | `0.9` / `0.22`                                   |
| Maximum speed / acceleration | `0.32` / `0.65` scene units per second / second² |
| Phase coupling               | `0.35`                                           |
| Random frequency drift       | `±0.065` cycles per second                       |
| Curl strength                | `0.12`                                           |

The ground is a configurable horizontal simulation plane in the scan's coordinates, not collision against a reconstructed terrain mesh. Soft forces turn particles before the bounds, with a final clamp keeping their bodies above the ground and below the ceiling.

On adapters supporting `float32-blendable`, the Gaussian color pass also writes
an `R32Float` occlusion attachment using minimum blending. Fragments with alpha
at least `0.3` contribute their original depth; other fragments contribute `1`.
Fireflies compare their fragment depth against this texture, preserving the
original `LessEqual` occlusion rule without rendering the forest twice. Devices
without that optional feature retain the separate `Depth32Float` pass. Both
paths use the existing approximate splat-center occlusion, alpha threshold and
sorted forest color blending. Color/depth caches invalidate together.

The moving-camera benchmark reached **63.19 FPS over 3,600 frames** at 1920 ×
1080 with 10,000 fireflies on the recorded local browser/GPU, using the merged
path. Two matched 1,200-frame runs averaged **66.66 FPS**, versus **38.61 FPS**
for the separate-pass control. These results cover the recorded route and
settings; see the moving-camera journal for raw data and discarded attempts.

Unlit fireflies return clipped vertices before the billboard transform, avoiding rasterization, glow fragment calculations and additive blending. Particle simulation and the instanced draw count are unchanged; even very faint positive flashes still render.

## Run

From the repository root:

```sh
pnpm install
pnpm --dir experiments/firefly_forest dev
```

Open <http://localhost:5178> in a WebGPU-capable browser. The 242 MB PLY loads once, then a Rust Wasm worker culls and sorts visible splats back to front when the camera moves. Loading status, visible splat count, sorting time and FPS appear over the scene.

- Drag to orbit.
- Shift + drag or middle drag to pan.
- Scroll to zoom.
- The **攝影機 / 視角 FOV** slider changes the vertical field of view from **20° to 100°**, in 1° steps, defaulting to **40°**. Changes immediately update the projection, Gaussian visibility and firefly occlusion.

The `dev` command builds optimized Wasm, starts Vite, and recursively watches `.rs` / `.wgsl` files in this experiment's `src/` and the shared `crates/belfast/src/`. Saving a file rebuilds release Wasm and reloads the page after a successful build. Saves within 200 ms are combined; edits during a build queue one more build instead of starting overlapping builds. Compilation errors leave the watcher running so the next save retries. Reloading resets the camera and firefly controls. TypeScript and CSS retain Vite's usual hot reload.

To watch and compile Wasm without starting Vite:

```sh
pnpm --dir experiments/firefly_forest watch:wasm
```

Both commands build once at startup and stop with Ctrl+C. Use `dev` at port **5178** for automatic page reloads; `watch:wasm` only updates `pkg/`, while `preview` at port **4175** serves the previously built `dist/`. Generated output is outside the source watch roots, preventing rebuild loops. Restart the watcher after changing Cargo dependencies. The one-time `build:wasm` and `build:wasm:release` commands remain available.

```sh
pnpm --dir experiments/firefly_forest build       # release Wasm, typecheck, static site
pnpm --dir experiments/firefly_forest preview     # http://localhost:4175
pnpm --dir experiments/firefly_forest test
cargo test -p belfast-experiment-firefly-forest
# GPU rendering and full local forest asset checks:
cargo test -p belfast-experiment-firefly-forest -- --ignored --nocapture
```

Deploy `dist/` as a static site, including `forest.ply`. For HTTPS preview on the same Wi-Fi, use `LAN_HOST=<your-local-IP> pnpm --dir experiments/firefly_forest preview:lan` and open port 5179. The receiving browser must trust the development certificate and support WebGPU.

## Structure

- `src/scene.rs`: forest renderer, orbit camera, frame updates and sort request lifecycle.
- `src/ply.rs`: binary little-endian 3DGS parsing, covariance and color packing.
- `src/splat.rs` / `src/splat.wgsl`: projected anisotropic Gaussian ellipses and alpha blending.
- `src/forest_color.rs` / `src/forest-color.wgsl`: same-format forest color cache and full-resolution copy.
- `src/sorting.rs`: frustum culling, radix depth sorting and camera motion threshold.
- `src/fireflies.rs`: particle initialization, tuning, compute dispatch and glow renderer.
- `src/firefly-{state,update,noise,draw}.wgsl`: particle layout, flocking/phase synchronization, curl field and emissive billboards.
- `src/browser.rs`: canvas/WebGPU setup and Wasm exports, including the CPU-only sorter.
- `web/main.ts`: asset loading, worker messages, browser input and cleanup.
- `web/sort.worker.ts`: sorts in a separate worker using the same Wasm module.
- `scripts/watch-wasm.mjs` / `scripts/wasm-watch.mjs`: Rust/WGSL watch, build queue and coordinated Vite reloads, matching `experiments/particles`.
- `src/input.rs` / `web/kinetic.ts`: the template's optional skeleton input hook for future interactions; no camera or tracking runtime is started.
- `pkg/` and `dist/`: generated output.

The loader supports standard uncompressed binary little-endian Gaussian PLY with XYZ, SH DC color, opacity, log scale and quaternion fields. Higher-order SH coefficients are not supported, matching the source renderer. Very low-opacity splats are filtered before upload; `forest.ply` contains 4,320,000 source splats and 4,116,349 visible-opacity splats.
