# Firefly forest

Renders `public/forest.ply` with the Rust/WebGPU 3D Gaussian splatting pipeline from `experiments/particles`. Original PLY colors are blended into the forest cache, then graded into a blue/teal night palette. The camera starts at **(0, 0, 5)**, looking toward **(0, 0, 0)** after converting PLY coordinates to Y-up. Orbital controls use the world origin as their look-at and rotation center, with an initial radius of **5 units**.

The entire scene is translated **0.5 units down**. The shared view transform includes this translation for splat rendering/sorting, firefly rendering/culling, fog and local-light reconstruction. Simulation bounds, ground height and lights remain in scene-local coordinates, so they move together. Benchmark camera snapshots include the translation, preventing comparisons against baselines with the old placement.

Press **F** to toggle fullscreen for the canvas only; **Esc** also exits.
The image keeps its 16:9 aspect ratio with black bars as needed, and controls stay
outside fullscreen. The shortcut is ignored while editing a control or text.

## Brush stroke rendering

**森林筆觸 / 使用短筆觸 splats** is enabled by default. Turn it off to compare
with the original Gaussian falloff at full count and original size. Brush mode
uses **2× width and height** and **50% of the original splats**. The worker keeps
one deterministic member of each original index pair before culling and sorting;
the subset stays fixed when moving the camera. Four short dry-brush masks from
[Dino0040's CC0 brushstroke pack](https://opengameart.org/content/100-grunge-brushstrokes-and-splatters-set)
replace Gaussian opacity; position, projected orientation, depth ordering and
forest grading remain intact. A stable hash of the original splat index chooses
the mask, flips and ±15% pigment variation, independent of sorting order.

The browser decodes the bundled PNGs once into padded 256 × 256 alpha layers.
Rust uploads an R8 texture array with averaged mipmaps (about 341 KiB) so distant
brush details filter smoothly. Both merged and separate occlusion passes use
the same brush alpha threshold, avoiding invisible Gaussian-shaped occluders.
The separate depth pass uses the same enlarged bounds as the color pass, and
worker frustum bounds expand by 2× too. Toggling requests a fresh sort and rebuilds
the forest color/depth cache; stale worker results from the other mode are ignored.
Firefly simulation and count are unchanged.
This is a per-splat texture experiment: tiny/subpixel splats will remain subtle,
and it does not merge strokes into the larger painted regions of the reference.
No performance benchmark has been run for this style.

Original PNGs and licensing information are in `public/brushes/`.

## 30-second flash rhythm

**螢火蟲 → 30 秒節奏** is enabled on page load, starting when the forest is
ready. The readout shows the current second and stage. Toggle it off to return
to the manual frequency, duration and randomness sliders; toggle it back on to
restart from zero without reseeding or moving particles.

| Seconds | Behavior                                                                     |
| ------- | ---------------------------------------------------------------------------- |
| 0–8     | Sparse independent flashes, short pulses and high individual variation.      |
| 8–16    | Local waves grow and travel from the rear of the forest toward the front.    |
| 16–22   | Local waves disperse into independent flashes and gradually fade away.       |
| 22–27   | Only a small stable subset can flash; the forest's base color stays visible. |
| 27–30   | Sparse individual flashes return smoothly into the next cycle.               |

The loop interpolates frequency, pulse duration, randomness and the fraction of
participating fireflies. World-space pulse delays provide
local propagation, then blend back to the existing independent oscillators as
participation falls over 16–22 seconds. There is no forest-wide synchronized pulse.
Individual oscillators keep running throughout the loop. Effective brightness is
written to the existing particle state, shared by particles, trail history and
forest lighting. Dragging retains the existing full-brightness override.

Timing follows rendered simulation seconds (background tabs pause/slow it),
independent of the motion slider. Benchmark reset restarts the loop and benchmark
v27 records whether it is enabled and separates this gradual fade from the older chorus rhythm. No extra GPU dispatch or particle buffer is
allocated. Tuning keyframes live in `src/firefly_rhythm.rs`; wave delay and pulse
spacing live in `src/firefly-update.wgsl`.

## Firefly trails

**螢火蟲 → 粒子 Glow** toggles the particle halo (off by default). Turning it off
retains the bright core, individual blinking and drag transitions. Trails and
forest lighting are unchanged; the toggle takes effect without resetting particles.

Under **螢火蟲 → 顯示**, **拖曳軌跡** is the default: idle shows glowing particles;
pressing the canvas starts a new trail and fades particles out over 0.25 seconds.
Releasing retracts the tail along the continuously updated curve toward the
moving particle head and fades particles back. Dragging also
smoothly brings each firefly to full brightness over **0.35–5 seconds**, using
independent fixed randomness with the height distribution's power-tail shape:
`mix(5, 0.35, r⁸)`. Most durations cluster near 5 seconds, with a few fast outliers;
brightness speed is unrelated to flight height. Each keeps its own progress, including when regrabbing during
recovery. This is independent of the 0.25-second particle/trail transition and
movement acceleration. On release,
that boost fades toward each firefly's current natural flash. **螢火蟲 → 閃爍恢復**
sets the shared duration for brightness recovery, trail retraction, particle fade-in
and movement slowdown from 0.1–5 seconds (0.65 seconds by default). Larger values
keep the whole transition visible longer. Changing recovery mid-release continues
from the current brightness, trail length and speed over the newly selected
duration; regrabbing also preserves the current transition.
The underlying cycles keep running; dragging never restarts or synchronizes them.
Particles and the whole corresponding trail share this effective brightness,
rather than replaying old tail flashes.
Motion uses
the existing 0.25-second smooth transition: it accelerates from the configured speed to 1×
while held, then returns to the configured speed on release. The slider value
and blink timing stay unchanged. Regrabbing during retraction continues from
the current speed, visible length and opacity. Pointer cancel,
lost capture and window blur all release the effect; scrolling alone does not.

The **粒子** and **軌跡 · 256 幀** modes retain manual comparisons. Trails follow
ribbon02's three-sided tube geometry, using up to 256 consecutive simulation
positions from a GPU ring buffer. A visible-age limit starts at zero, grows one
sample per rendered frame while held, and smoothly decreases during release.
Fractional sampling at the tail prevents whole-segment jumps. At idle all visible
nodes resolve to the current head and the trail draw is skipped, so stale
pre-drag history never appears without rewriting all 409.6 MB each idle frame.
Startup, increased particle counts and benchmark resets still initialize every
history slot at the current positions.

Trails use unlit colors from the existing firefly palette, with ordinary alpha
blending and no halo or additive glow.
Manual trail mode stays solid; drag mode combines the live flash envelope
with the shared brightness boost using the history sample's fourth component.
The simulation keeps individual brightness progress in particle state; the
history sample stores its effective brightness without adding history storage.
**大小** controls their thickness. Forest
depth still occludes them, and a separate cleared depth buffer handles trail
overlap without changing the cached forest. During transitions, a depth prepass
makes the nearest surface blend once, independent of particle order. Fully held
and manual solid trails retain the single draw pass. Trail length depends on movement
speed and FPS; there is no extrapolation or time stretching.
History reserves 409.6 MB for the 100,000-particle capacity, split into four
64-frame buffers of 102.4 MB to stay within default WebGPU binding limits.
Recording is included in
the simulation GPU timing; the draw timing measures the selected mode.

The previous [256-frame measurements](benchmarks/2026-10-23-trails-256/README.md),
before drag transitions were added, reached
94.80 FPS static / 65.29 FPS moving at 20,000 particles, and 21.05 / 19.91 FPS at
100,000 particles in single 1,200-frame local runs at 1920 × 1080.

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
  (0–2×, default 0.55×), foliage color (`#375e77`), tree/shadow color (`#191f33`)
  and highlight color (`#3e4f6a`).
- Firefly main, transition and warm colors; warm-color share (0–100%);
  minimum and maximum peak multipliers (0–3×). Equal limits remove individual
  peak variation. Moving one limit across the other moves the other with it.
  Default colors are `#efff14`, `#ffe224` and `#ff9c14` respectively.
- **恢復調色預設** restores the current night palette and 0.45–1.4× peak range,
  without changing simulation controls or the camera. Reload also restores defaults.

Changes update two small uniform buffers immediately, with no shader rebuild,
particle reset or forest-cache rebuild. Color pickers use display RGB hex colors.
Benchmark v27 records all color, local-light and fog settings (including noise frequency), firefly randomness and height settings, sprite size, shared flash/trail recovery time, particle glow, 30-second rhythm, particle/trail/drag display and brush-rendering mode (2× size / 50% count), and locks these controls during a run. Different recovery times, particle glow or rhythm settings are incompatible baselines. In drag mode the scripted moving path holds the effect active at 1× after acceleration, while a static run shows particles at the configured speed; completion or cancellation releases it. The Benchmark panel is collapsed by default.

## Animated volumetric fog

The **體積霧** controls toggle fog, density (0–0.2, default 0.06), animation
speed (0–2×, default 1) and light intensity (0–4×, default 0.7). Speed zero freezes
the density field. **霧的 Noise 頻率** (0.1–4×, default 1, step 0.05) scales both
noise octaves together: lower values create broader fog patches, higher values
create finer detail. It preserves their frequency ratio and the world-space wind
speed; 1× preserves the original noise scale. Changes take effect immediately
without rebuilding the noise texture. A blue light at **(0, 2.2, -10)** shines from the brighter
front opening toward the foreground. The position and cone direction are in
`src/fog.rs` and `src/fog.wgsl`; these are scan units rather than meters.

Each frame ray marches at **480 × 270** for a 1920 × 1080 canvas (one quarter
in each dimension), with **32 midpoint samples** over at most 32 scene units.
The existing forest depth stops the camera ray at a visible surface. A seeded
32³ noise texture, sampled at two scales with slow world-space wind, animates
the density without CPU texture uploads. Height falloff, Beer–Lambert
transmittance and forward scattering give the light volume depth and motion.
Density is concentrated near the firefly ground plane **Y = -0.45**:
`0.2 + 2.3 * exp(-heightAboveGround / 0.8)` multiplies the animated noise.
It reaches **2.5× at ground level**, fades to about **1.05× at 0.8 units above
ground** and **0.51× at 1.6 units**, with a 0.2× haze remaining aloft. The
height profile stays anchored as the noise moves, so fog feels settled along
the ground. This uses the simulation's flat ground approximation, not sampled
terrain elevation. The density slider scales the whole profile. Sampling
resolution, ray-march step count and texture reads are unchanged.
Depth-aware four-tap upsampling composites scattering/transmittance in the
existing forest-copy pass, after grading. Fog animation does not invalidate
the static forest cache. Additional textures occupy about **1.52 MiB** at 1080p.

Firefly local lighting is **on by default** and can be toggled separately.
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
Those measurements predate the denser ground profile.

## Firefly light volume

The **螢火蟲照亮森林** controls enable local lighting (on by default), set
strength (0–1×, default 0.35), world-space radius (0.25–2, default 0.9), and
update frequency (1–120 Hz, default 60 Hz, or **每幀更新** at the slider's
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

Accumulated forest illumination has a hue-preserving soft ceiling after the
strength control: peak RGB values up to **0.25** are unchanged, then smoothly
approach **0.5**. Dense clusters therefore stop getting arbitrarily brighter;
the particle cores/glow, trails and fog are unaffected. This ceiling is set by
`FIREFLY_LIGHT_LIMIT` in `src/forest-color.wgsl`, before surface-color modulation.

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
at world origin (separate from the interactive starting position); FOV remains the chosen control value. Disable this option
to benchmark the current static view. Completion or cancellation restores the
interactive view. Path version, sorting cadence, color, local-light and fog settings are part of
baseline compatibility; v27 baselines record the rhythm toggle and are stored separately from earlier v1–v26 results, which used different rhythm timing.
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

The **粒子數量** (particle count) slider selects **10,000–100,000 fireflies**, in steps of 1,000, defaulting to 20,000. Both particle buffers and grid links are allocated for 100,000 particles at startup; changing the active count updates compute dispatches, neighbor lookup bounds and draw instances without reallocating buffers or resetting active particles. Inactive particles are excluded from simulation and rendering.

Fireflies run entirely on GPU compute. Each frame clears a spatial grid and inserts every active particle into an atomic linked list. Particles within the camera frustum and its guard band search the 27 adjacent cells for neighbors. The lists have one entry per particle, so crowded cells do not silently drop neighbors. Particle state alternates between two storage buffers; rendering uses the latest output without CPU readback.

The compute shader skips neighbor searches, flocking forces and phase synchronization outside the expanded frustum. The guard band is one neighbor radius plus `0.1 × size multiplier` scene units for the glow billboard (**0.975 units** with the default settings), measured against normalized world-space planes and updated with the camera every frame, including FOV and firefly size changes. Offscreen particles remain in the grid so visible particles can still interact with them. They continue moving with curl noise, drag and boundary constraints, and advance their own randomized cycles and fixed-duration flashes; neither state nor time freezes when leaving the view.

Nearby fireflies separate, more distant neighbors attract, and their velocities align. Analytic curl noise adds wandering, with an individual sampling offset and animation rate so adjacent fireflies do not follow identical paths. This reuses one curl evaluation per particle. Motion integrates acceleration into velocity, then velocity into position using small time steps, drag and a speed limit.

Each firefly has a continuous phase in `[0, 1)` and its own intrinsic frequency between 0.16 and 0.28 cycles per second at 1× randomness. Neighbor differences use `fract(neighborPhase - phase + 0.5) - 0.5`, the signed shortest distance around the cycle. A positive average speeds the oscillator up; a negative average slows it down. Weaker coupling preserves individual timing alongside local synchronization. Each particle also has smooth random frequency drift, up to ±0.065 cycles per second at 1× randomness, with an individual offset and rate. The phase never jumps to match a neighbor.

The **移動隨機程度** and **閃爍隨機程度** sliders range from **0–2×**, default **0.60×**. Movement randomness scales the curl force while preserving separation, cohesion and alignment. Blink randomness scales both the intrinsic-frequency spread around 0.22 cycles/second and the smooth frequency drift, leaving neighbor coupling and flash duration unchanged. Zero removes these disturbances, but does not reset existing velocities or phases, so particles do not instantly align or synchronize. These controls preserve seeded randomness and take effect without restarting the simulation.

The **螢火蟲大小** slider ranges from **0.25–3×**, default **0.75×**, scaling each billboard's core and halo while retaining individual size variation. It also expands the offscreen simulation guard band to cover larger sprites. Physical boundary constraints and the separately controlled local-light radius stay unchanged.

Each cycle wrap triggers a separate flash, with a smooth rise and fall and no glow between flashes. The **閃爍時長** (flash duration) slider selects **0.1–2 seconds**, in 0.05-second steps, with a **1.5-second default**. The GPU captures the selected duration at the start of each flash and tracks its age in unscaled seconds, including the fraction of the triggering frame after the cycle wraps. Duration changes apply to the next flash, so active pulses do not jump and expired pulses stay dark. Frequency, neighbor synchronization and jitter only affect the interval between flashes; even a frequency change during a flash leaves its duration unchanged.

The on-screen **移動速度** (movement speed) and **閃爍頻率** (blink frequency) sliders independently scale motion and oscillation from `0` to `1`, defaulting to **0.27×** and **0.28×**, respectively. `1` preserves the original speed; `0` pauses motion or prevents new flashes without resetting the cycle. An active flash finishes normally even at frequency `0`. Noise animation clocks use the square root of their respective slider rates, so wandering and frequency drift keep evolving at low settings without speeding up motion or cycle integration. Zero still freezes the corresponding noise clock. Changes take effect immediately without reloading the forest.

Tune height in **螢火蟲 → 高度**. The seven live controls are **基準高度**, **隨機幅度**, **高飛加成**, **分布次方**, **高度上限**, **Noise 尺度**, and **Noise 幅度**. Larger noise scale makes broader regions; larger exponent makes high outliers rarer. Changes preserve existing particles, their individual random seeds, flash cycles and trail history. The upper-limit slider supports `0.25–3.0`; the full grid/light bounds remain at `3.0`. Benchmark runs lock and record these controls.

Defaults live in `src/firefly_height.rs` and `Settings::default()` in `src/fireflies.rs`:

| Parameter                    | Default                                                 |
| ---------------------------- | ------------------------------------------------------- |
| Population                   | `20,000` default, `10,000` minimum, `100,000` maximum   |
| X / Z bounds                 | `[-10, 10]` / `[-24, 6]`                                |
| Ground / outer ceiling Y     | `-0.45` / `3.0`                                         |
| Individual base ceiling Y    | `1.25–3.0`, median approximately `1.5`                  |
| Ceiling noise                | XZ cell size `4.0`, amplitude `±0.5`, outer limit `3.0` |
| Neighbor / separation radius | `0.9` / `0.22`                                          |
| Maximum speed / acceleration | `0.32` / `0.65` scene units per second / second²        |
| Phase coupling               | `0.35`                                                  |
| Random frequency drift       | `±0.065` cycles per second                              |
| Curl strength                | `0.12`                                                  |

Each particle keeps a fixed random value `r`; its base ceiling is `base + spread × r + outlier × r^exponent`, capped by the height limit. Defaults give `1.25 + 0.5r + 1.25r⁸`. Most base ceilings lie between `1.25` and `1.75`, with a sparse tail approaching `3.0`. Static 2D value noise sampled at the particle's current XZ adds up to `±0.5` height variation, with four-unit cells and smooth quintic interpolation. The final ceiling is clamped between `0.25` and the selected height limit to stay safely above the ground. CPU spawning, benchmark reset and GPU simulation use the current controls and the same surface; the fixed random value stays in the existing particle state, and the neighbor grid and light volume retain the full outer bounds.

The ground is a configurable horizontal simulation plane in the scan's coordinates, not collision against a reconstructed terrain mesh. Soft forces turn particles before the bounds, with a final clamp keeping their bodies above the ground and below their individual ceilings.

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
- The **攝影機 / 視角 FOV** slider changes the vertical field of view from **20° to 100°**, in 1° steps, defaulting to **50°**. Changes immediately update the projection, Gaussian visibility and firefly occlusion.

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

To rebuild the Wi-Fi version automatically while editing Rust or WGSL, use one
combined watcher instead of running `dev` and `preview:lan` separately:

```sh
LAN_HOST=<your-local-IP> pnpm --dir experiments/firefly_forest dev:lan
```

This builds release WASM, checks TypeScript and bundles `dist/` at startup and
after `.rs` / `.wgsl` saves in the experiment or shared Belfast source. The same
queue coalesces saves and serializes builds. It serves local development at
`http://localhost:5178` and the built HTTPS preview at `https://<your-local-IP>:5179`.
The local page reloads after each successful build; refresh the page on other
computers after `[lan] Updated dist/` appears. Ctrl+C stops both servers and the
watcher. Frontend-only edits still use local HMR; restart `dev:lan` to bundle them
for Wi-Fi. No tests run as part of this command.

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
