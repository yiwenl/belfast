# Firefly forest

Renders `public/forest.ply` with the Rust/WebGPU 3D Gaussian splatting pipeline from `experiments/particles`. Original PLY colors are blended into the forest cache, then graded into a blue/teal night palette. The camera starts at the scan origin, looking along negative Z after converting PLY coordinates to Y-up.

## Render resolution and static forest cache

The drawing buffer is fixed at **1920 × 1080**, independent of device pixel ratio. CSS fits the canvas to the window at 16:9 without stretching it; pointer input remains relative to the displayed canvas.

The static forest color and occlusion depth are cached together. Camera, FOV, viewport, asset or sorted-index changes invalidate both; otherwise a full-resolution texture copy precedes the animated firefly pass. The cache uses the selected surface format, preserving HDR values and the existing forest blending. At 1080p, the RGBA16Float color texture adds about **15.82 MiB** of GPU memory.

This optimization benefits a stationary camera. While the camera moves, the forest still renders every changed frame and also incurs the copy pass; the fixed-camera benchmark does not establish moving-camera performance. Firefly simulation and glow continue every frame.

Forest splats whose centers are within **0.5 scene units in front of the camera**
are clipped in the vertex shader, before covariance projection. From **0.5 to
1.0 units**, opacity smoothly increases to its original value to avoid a hard
pop as the camera moves. Both merged and separate occlusion passes use this
faded opacity, including the separate depth pass's ellipse bounds, so hidden
splats do not leave invisible occluders or near-surface light-map samples.
The distances use camera-space forward depth and can be adjusted with
`NEAR_HIDE_DISTANCE` / `NEAR_VISIBLE_DISTANCE` in `src/splat.wgsl`. Firefly
billboards are unaffected; the worker's visible count still includes these
near-camera splats because clipping happens on the GPU.

## HDR output

The renderer requests HDR through Belfast's surface selection helper. Supported browsers use an `rgba16float` canvas with `ExtendedSrgb` (`toneMapping.mode: "extended"`), preserving additive firefly emission above SDR white instead of clamping it to `1.0`. The firefly shader's base peak multiplier is **3.0** for HDR, compared with **1.8** for the SDR fallback, before applying each insect's brightness variation. The existing flash envelope and timings are unchanged.

The browser canvas expects encoded extended sRGB. The forest cache retains its original display RGB and existing blending; the final copy grades it before adding HDR fireflies. The overlay reports **HDR 輸出** or **SDR 輸出** from the selected surface mode; actual highlight headroom depends on the display and OS settings. Browsers without fp16 canvas support retain SDR output.

## Night palette

`src/forest-color.wgsl` grades the already-composited forest in its existing
full-screen copy, so there is no extra pass or per-splat grading work. Source
green dominance softly selects teal foliage; other colors and deep shadows
shift toward navy, with subdued blue highlights. A luminance curve preserves
texture while bringing the daylight scan into a night palette. The selection
uses color rather than semantic tree/leaf labels. Hardware-sRGB attachments
convert to display RGB before grading and back afterward; HDR/unorm targets
already use display RGB.

`src/firefly-draw.wgsl` keeps additive HDR emission after forest grading. A
stable integer hash of particle index supplies independent hue and peak-light
variation: mostly yellow-green, occasional amber, and peak multipliers from
0.45 to 1.4, biased toward dimmer insects. Cores retain some of each insect's
hue. This changes appearance only; cycle timing, flash duration, movement,
billboard size and occlusion remain unchanged. No texture or simulation-buffer
allocation is added. Earlier optimization journals measure the pre-grade look.

The right-hand panel has collapsible **森林調色** and **螢火蟲色彩** controls:

- Forest grade strength (0–100%; 0 restores original colors), brightness
  (0–2×), foliage color, tree/shadow color and highlight color.
- Firefly main, transition and warm colors; warm-color share (0–100%);
  minimum and maximum peak multipliers (0–3×). Equal limits remove individual
  peak variation. Moving one limit across the other moves the other with it.
- **恢復調色預設** restores the current night palette and 0.45–1.4× peak range,
  without changing simulation controls or the camera. Reload also restores defaults.

Changes update two small uniform buffers immediately, with no shader rebuild,
particle reset or forest-cache rebuild. Color pickers use display RGB hex colors.
Benchmark v6 records all color, local-light and fog settings and locks these controls during a run.

## Animated volumetric fog

The **體積霧** controls toggle fog, density (0–0.2, default 0.06), animation
speed (0–2×, default 1) and light intensity (0–4×, default 0.7). Speed zero freezes
the density field. A blue light at **(0, 2.2, -10)** shines from the brighter
front opening toward the foreground. The position and cone direction are in
`src/fog.rs` and `src/fog.wgsl`; these are scan units rather than meters.

Each frame ray marches at **480 × 270** for a 1920 × 1080 canvas (one quarter
in each dimension), with **32 midpoint samples** over at most 32 scene units.
The existing forest depth stops the camera ray at a visible surface. A seeded
32³ noise texture, sampled at two scales with slow world-space wind, animates
the density without CPU texture uploads. Height falloff, Beer–Lambert
transmittance and forward scattering give the light volume depth and motion.
Depth-aware four-tap upsampling composites scattering/transmittance in the
existing forest-copy pass, after grading. Fog animation does not invalidate
the static forest cache. Additional textures occupy about **1.52 MiB** at 1080p.

Firefly local lighting is **off by default** and can still be enabled separately.
Fog does not sample the firefly light volume; sprites retain their existing
emission and are drawn afterward. This first version uses approximate splat
surface depth and an unshadowed broad light cone, without light-space occlusion,
multi-scattering or fog attenuation of firefly sprites. Thin geometry may show
depth-upsample artifacts. It is an artistic approximation in the renderer's
existing display-RGB pipeline, not calibrated physical lighting.

The fog clock resets with the benchmark seed and advances by fixed simulation
steps during measurement. Its GPU work is included in **森林＋光照＋霧 ms** and
total GPU time. Disable **啟用體積霧** to compare against the same scene without
ray marching. The design targets an approximately 10 FPS cost; use matched
moving-camera measurements to establish the actual cost on a given GPU.
The [first fog measurements](benchmarks/2026-10-20-volumetric-fog/README.md)
record **68.73 → 65.92 FPS** with fog enabled, a **2.82 FPS** drop and
**0.49 ms** additional whole-frame GPU time for the paired route/settings.

## Firefly light volume

The **螢火蟲照亮森林** controls enable local lighting (off by default), set
strength (0–1×, default 0.1), world-space radius (0.25–2, default 0.9), and
update frequency (1–120 Hz, default 30 Hz, or **每幀更新** at the slider's
rightmost position). Every-frame mode samples the latest particle state each rendered
frame, keeping lighting in step with the sprites; frequency is capped by actual
rendered FPS. Benchmark metadata uses `hz: 0` for this mode. Radius uses the
scan's scene units, not meters. A lower frequency saves updates but makes light
transitions more stepped; for short 0.1-second flashes, increase the update
frequency to avoid missing flashes.

`light_volume.rs` renders the latest simulated particle buffer into a real
**96 × 128 × 32 RGBA16Float 3D texture**, using X/Z/Y axis order. Four-vertex quads cover spherical light cross-sections in Y slices, reject
unlit/out-of-slice particles in the vertex shader, and use One + One additive
blending with a smooth finite-radius falloff. wgpu 30 currently drops
`depth_slice` in its WebGPU backend, so blending uses a 2D array of slices;
a single compute pass copies them into the 3D texture for trilinear sampling. More
nearby insects contribute more light; there is no density normalization. All
slices clear on each update so old light does not accumulate over time. The
volume plus staging array cost **6 MiB** and include a 2-unit border around
simulation bounds. The staging workaround can be removed after a wgpu backend
upgrade that correctly forwards `depth_slice`.

Billboards and the light volume share `firefly-appearance.wgsl`, including each
insect's stable hue, peak multiplier and current fixed-duration flash envelope.
There is no CPU particle readback. Updates use simulation time (the benchmark's
fixed timestep), with cadence reset alongside the seed. Disabling lighting
skips volume generation; enabling it or changing update frequency, radius, count or firefly
appearance forces a fresh volume. Strength changes apply immediately.

The existing forest copy pass reconstructs world coordinates from cached depth
and the inverse camera matrix, trilinearly samples this volume, and adds warm
light modulated by the source forest color **after** night grading. No extra
full-screen pass or static forest-cache invalidation is required for changing
lights. Both merged R32Float and fallback Depth32Float paths are supported.
Volume generation is included in the benchmark's forest GPU span and whole-frame
timing; control values are recorded and locked during measurement.

This is an artistic, isotropic light field: it has no surface normals, shadows
or light occlusion, so light can bleed through nearby geometry. The existing
alpha-thresholded splat-center depth approximates one surface, not all translucent
Gaussian layers. Offscreen insects still contribute their simulated emission.
A held low-rate light snapshot may lag the visible sprites by one update interval.
The previous 60 FPS optimization results predate this feature; use the benchmark
to measure its cost for the chosen particle count and radius.

## Manual benchmark

Open the **Benchmark** panel to select a seed (`1–4294967295`, default
`1831565813`) and measurement length (`1–10000` frames, default `1200`).
The default **固定移動路徑** option runs `forest-loop-v1`, a 20-second loop
combining translation, yaw and pitch. Each camera pose uses the fixed simulation
frame index, so slower rendering does not change the route. The camera starts
at the scan origin; FOV remains the chosen control value. Disable this option
to benchmark the current static view. Completion or cancellation restores the
interactive view. Path version, sorting cadence, color, local-light and fog settings are part of
baseline compatibility; v6 baselines are stored separately from earlier v1–v5 results.
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
forest color/depth rendering including cache rebuild/copy and light-volume updates, and firefly glow rendering. GPU whole-frame time
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
- `src/forest_color.rs` / `src/forest-color.wgsl`: forest cache, night grading and depth-reconstructed local lighting.
- `src/light_volume.rs` / `src/light-volume*.wgsl`: additive 3D light field and update cadence.
- `src/fog.rs` / `src/fog*.wgsl`: animated low-resolution volumetric scattering and depth guide.
- `src/firefly-appearance.wgsl`: shared particle hue and emission for sprites and illumination.
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
