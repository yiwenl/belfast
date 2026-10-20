# Particle experiment

An independent experiment copied from `../template`, using Rust/Wasm for simulation and rendering with a Vite + TypeScript host.

Loads `public/courtyard-foreground.ply` as a Gaussian splat scene with an independent Rust sky background and 10,000 GPU-simulated magnolia petals. The original `public/courtyard.ply` is preserved; use **View original** or `?tree=original` to compare. Rust owns asset decoding, rendering, the camera, particle simulation and depth sorting. The Vite host handles loading, DOM events, and a Worker running the Rust/Wasm sorter. FPS, the Kinetic input API, and optional HTTPS LAN preview are retained; `../template` remains the reusable starter.

## Magnolia model

The default scene also loads `public/magnolia-branches.glb`: 38,624 vertices / 57,852 triangles, with its embedded bark base-color and normal textures. Vite fetches the bytes; Rust decodes and renders the model. `?tree=original` omits the model so the original scanned tree remains a useful comparison. `?sky=original` retains the replacement model.

The GLB keeps its Y-up orientation. Its node transforms are baked once, then its bounds are uniformly fitted to **2.8 scene units high**. The horizontal bounds center / lowest vertex is placed at **`(0.05, -0.80, -5.98)`** in render coordinates (original PLY `(0.05, 0.80, 5.98)`), on the central grass where the old tree stood. Adjust `MAGNOLIA_BASE` / `MAGNOLIA_HEIGHT` in `src/scene.rs` to change placement or size.

Sky draws first; the opaque mesh writes a depth buffer; sorted splats then test against that depth without writing it. Splats in front of the branches blend over them, while splats behind them are hidden. This uses each splat's center depth, so intersections with broad Gaussians are approximate. Mesh vertices are uploaded once and never enter the splat sorter. The depth target resizes with the canvas.

The loader supports static opaque triangles, node transforms, normals, UV0, embedded PNG/JPEG base-color and normal textures. It rejects unsupported skins, morphs, transparency and required extensions. Bark and petals use a small 0.05 ambient term plus the shadowed courtyard point light, repeat/linear texture sampling and no mipmaps; this is not a full glTF PBR renderer. It does not cast a new shadow onto the baked scan, and changing the sky does not relight the model.

```ts
app.loadGlb(glbBytes); // Uint8Array; replaces the model at the configured courtyard placement
```

```sh
cargo test -p belfast-experiment-particles --test model -- --include-ignored
```

## Magnolia petals

The default scene fetches `magnolia-petal.glb` and `magnolia-emitter-positions.json` alongside the branches and courtyard. Rust parses all 1,435 flower/bud positions in tree-local Y-up coordinates and applies the **same branch placement matrix**. Do not apply the JSON's Blender conversion again. The original-tree comparison skips both replacement models and the emitters.

- The **Petals** selector switches between **10,000 (default), 100,000 and 500,000 opaque instances**. They share the petal's 45 vertices / 64 triangles and embedded ivory texture: one indexed instanced draw for its single primitive (640,000 / 6,400,000 / 32,000,000 triangles). Both sides render, normals rotate with each petal, and the shared GLB material shader receives the courtyard point light.
- Changing count resets the petals with randomized initial life, size and speed, replacing both GPU state buffers and their compute/draw bindings. Meshes, textures, pipelines, the camera and splat sorting state are retained. Each state is 80 bytes per petal; the buffer pair uses 1.6 / 16 / 80 MB. Reloading the page returns to 10,000.
- The **Scale** slider adjusts global petal size from **0.2× to 1.0×**, in 0.01 steps (default 0.5×). It multiplies each petal's random size and lifetime envelope in the vertex shader. Changes update a small uniform without resetting simulation; changing the count retains the chosen scale.
- Initialization distributes particles over all emitters with a small fixed offset (at most `0.03` scene units). Geometry uses the tree's uniform scale multiplied by an independent random **0.45–1.65×** size. Random unit axes, signed angular speeds and starting angles make petals tumble independently.
- Each petal has a 6–12 second total lifetime, with its **initial remaining lifetime randomized across the full cycle**. Its initial scale matches that age, so it is visible immediately and roughly 80% of petals can begin moving on the first compute frame. Initial positions stay near their emitters and initial velocities are zero; no simulation warm-up is required. During the first **20%** of each life it stays at its emitter with its initial angle; wind, curl and rotation begin only after that hold. Respawn starts a full new life. A frame crossing the hold boundary integrates only the time after release.
- Scale eases from zero to full during the first **10%**, stays full through 80%, then eases to zero over the final **20%**. This multiplies each petal’s original random size in the vertex shader; no alpha blending or particle sorting is added. The original mesh normals remain unit length even when the geometry shrinks.
- The compute shader uses `velocity += force * h`, then `position += velocity * h * speedMultiplier`, then `velocity *= pow(friction, h * 60)`. Friction defaults to `0.985` retained velocity per 1/60 second. Frames are divided into steps no larger than 1/120 second; elapsed time remains capped at 0.1 seconds after interruptions. This keeps Euler integration close across display frame rates.
- Each petal keeps an independent **0.6–1.6× movement speed multiplier**, applied only to displacement. It does not compound velocity or change lifetime/rotation timing. Speed and base size remain fixed across respawns.
- Force combines upward wind `[0.12, 0.22, -0.04]` with animated **3D curl noise**. The curl is derived analytically from a smooth quintic value-noise vector potential using eight lattice samples, avoiding repeated finite-difference noise evaluations. It is not normalized, so it retains the curl field’s zero-divergence property. Defaults: spatial frequency `1.5`, force strength `0.08`, time speed `0.18`. The simulation clock advances only when a compute frame is encoded.
- Expired petals return to their own fixed emitter-plus-offset position, reset velocity/angle/scale and restore their full lifetime, including the hold and scale-in stages. The count stays constant. There is no collision simulation yet.
- Two 80-byte-per-particle storage buffers alternate: compute reads A and writes B, then both shadow and visible passes read B in the same submitted command encoder. Only small uniforms are updated from the CPU; particle state stays on the GPU. At the default 10,000 particles the buffers together occupy 1.6 MB.
- Petals write depth alongside the branches, before transparent splats. They need no sorting and remain independent of the **Normal resort / Sort once** comparison. Existing splat center-depth approximation still applies at intersections.

Tune count, spawn spread, size/speed ranges, wind, friction, hold/scale fractions, curl parameters, lifetime and spin in `src/petals.rs`; integration/respawn are in `src/petal-update.wgsl`, the curl field in `src/petal-noise.wgsl`, and instanced transforms in `src/petal-draw.wgsl`. Existing `.rs` / `.wgsl` watching rebuilds these automatically.

```ts
app.loadGlb(branchBytes);
app.loadPetals(petalBytes, emitterJsonBytes); // Uint8Arrays; returns 10,000
app.setPetalCount(100_000); // 10_000 | 100_000 | 500_000; returns the active count
app.setPetalScale(0.5); // 0.2–1.0; returns the active scale
// Reloading branches clears the old petals; call loadPetals again to realign them.
```

```sh
cargo test -p belfast-experiment-particles --test petals
cargo test -p belfast-experiment-particles --lib petals::tests -- --include-ignored
```

## Courtyard point light

`COURTYARD_LIGHT` in `src/scene.rs` places a warm-white point light at render coordinates **`(0.05, 25.0, -5.98)`**, above the complete petal flight envelope. Its linear RGB color is `[1.0, 0.88, 0.66]`; intensity is `600.0` to compensate for the increased distance. Adjust those fields to move or tune it. The model receives diffuse light using its normal map with softened inverse-square attenuation `intensity / (1 + distance²)`, added to the existing ambient/directional fill.

A solid, unlit **yellow sphere** marks the exact same position; `LIGHT_MARKER_RADIUS` controls its size (default `0.25`). It writes depth alongside the model before splats draw, so foreground splats can occlude it. The marker is far above the initial 40° view: tilt the camera upward to see it. Neither the marker nor the light enters the splat sorter.

### Petal and branch shadows

Rust uses the same Belfast `WGSL_SHADOW_PCF_3X3` helper as `web/particles`, with a **2048² Depth32Float map** and strength **0.85**. Each frame runs compute, renders branches and petals into the depth map, then draws the visible scene. Both sides of the thin meshes cast shadows. `petal-transform.wgsl` is shared by both passes, including random scale, lifetime scale, the live scale slider, position and rotation. Count changes reuse the shadow map and pipelines.

The shadow camera is perspective and starts at the point light's exact position. At load time it fits the branches and a compact flight envelope: emitter placement, full petal geometry at maximum random size, spawn spread, maximum lifetime/speed, wind/friction travel, plus **1.5 world units of curl wandering margin** (`SHADOW_CURL_MARGIN` in `src/petals.rs`). This replaces the previous worst-case assumption of peak curl force acting in one direction for an entire lifetime. The current envelope is about **8.3 × 8.5 × 5.3** world units, down from **23.9 × 22.5 × 22.3**. It is a practical quality envelope, not a mathematical bound on every possible future noise trajectory; the GPU flight test checks complete maximum-speed/lifetime flights at five noise times. Recheck that test and adjust the margin after changing the motion settings.

The projection fits all four sides independently, with a 2% image-space guard and 0.15 world units beyond each depth limit. The camera stays fixed during animation, avoiding shadow swimming or per-frame CPU readback. The light must remain above the configured envelope. A cyan Belfast `CameraHelper` draws the exact same view-projection frustum over the scene, including near/far planes; its lines never cast shadows.

PCF attenuates the point-light contribution while preserving a small 0.05 ambient term. There is no separate directional fill to wash out the shadows. A normal offset, raster depth bias and small comparison bias reduce self-shadow artifacts. Tune resolution/strength and bias in `src/shadow.rs`. The extra depth pass adds one instanced petal draw, so 500,000 petals render 32 million triangles in each of the shadow and visible passes. Gaussian splats retain their baked lighting and neither cast nor receive these new shadows.

The lower-left **Shadow map** debug panel displays the exact 2048² texture used by PCF, live after the scene pass. It shows raw, nonlinear depth with no contrast remapping: black is near, white is far or empty. The entire light view is shown, so a tiny cluster in the center reveals how little of the map is occupied by actual casters. Rust reads the depth texture directly on the GPU; a normalized DOM rectangle keeps the overlay aligned through resize/DPR changes without CPU pixel readback. The panel is hidden for `?tree=original`.

## Central tree removal

`courtyard-no-tree.ply` removes 38,723 records (28,199 above the renderer's alpha cutoff), leaving 4,281,277 records / 3,504,888 active Gaussians. Every retained record is byte-for-byte identical to the original, including color, opacity, scale, and rotation; only selected records and the header vertex count change.

The selection was measured from front, side and top projections in original PLY coordinates. The crown/upper trunk volume is X `[-1.65, 1.70]`, Y `[-2.60, 0.72]`, Z `[5.10, 6.80]`. At the root, selection narrows to an ellipse centered at X `0.05`, Z `5.98`, with radii `0.23 / 0.24`, down to Y `0.80`. This retains the surrounding grass, paving and buildings. It is a mask for this scan, not a general semantic tree detector.

The edit deletes geometry without synthesizing hidden surfaces. The existing scan covers the background well from the initial view; other viewpoints may reveal original reconstruction gaps. Ground texture/shading is preserved, including any baked-in tree shadow or root discoloration.

To regenerate with Python 3 (standard library only):

```sh
python3 experiments/particles/tools/remove_courtyard_tree.py --overwrite
python3 -m unittest discover -s experiments/particles/tools -p 'test_*.py'
```

Adjust `CROWN` / `ROOT` in the script to refine selection. The script refuses in-place input replacement and existing output unless `--overwrite` is provided. Counts, bounds and source/output SHA-256 values are in `assets/courtyard-tree-removal.json`. The default and original variants each load into a fresh App/Worker pair when switched, preserving the one-model lifecycle.

## Independent sky

The default foreground excludes 1,116,901 sky/edge records, including **499,603 active splats**. The foreground contains 3,164,376 records / **3,005,285 active splats**: 14.25% fewer active entries than the no-tree scene. Download size drops from 239.8 MB to 177.2 MB and immutable splat textures shrink by about 16.0 MB. These are data reductions, not a guaranteed FPS improvement; visible counts, transparency overdraw and device performance depend on viewpoint.

- **Day / Night** switches an infinite-distance gradient background. **Load panorama** accepts a local 2:1 PNG/JPEG/WebP, at most 32 MB and 8192 × 4096 pixels. Dimensions are checked before decoding; the browser decodes to at most 4096 × 2048 for upload. The file stays in the browser; it is not uploaded to a server. Reloading clears custom images.
- Rust draws one opaque fullscreen triangle before foreground splats. It uses camera rotation and projection, ignores translation, and never enters the sort Worker. Changing sky does not trigger sorting or re-upload splat attributes. Foreground sorting retains the existing camera thresholds.
- Panorama center faces original **+Z**, image right turns toward **+X**, image top faces original **-Y**. The Wasm API accepts display-RGB RGBA8 (not HDR/EXR); `setSkyImage` validates dimensions/byte length before creating a GPU texture.
- `?sky=night` starts with the dark sky; **Compare scanned sky** / `?sky=original` loads the no-tree scan with its original sky and disables the replacement background. `?tree=original` compares the full untouched scan, including its sky.
- `assets/courtyard-sky.ply` archives the selected sky splats outside `public/`, so it is not downloaded or copied into the deployed site. `public/courtyard-no-tree.ply` and `public/courtyard.ply` remain untouched. Both new partitions copy all vertex bytes in original order. Counts and hashes are recorded in `assets/courtyard-sky-separation.json`.

The separation combines a measured roof envelope with color/scale cleanup of bright sky residue, protecting the two pale central facades. It is a **scene-specific heuristic**, not semantic segmentation: a few mixed roof/sky splats or fringe highlights can remain, especially from other viewpoints. Original daytime lighting and shadows are baked into the scene colors; switching to a night sky does not relight the courtyard. Refine the selection in `tools/split_courtyard_sky.py` if the experiment needs closer roof views.

```sh
python3 experiments/particles/tools/split_courtyard_sky.py --overwrite
python3 -m unittest discover -s experiments/particles/tools -p 'test_*.py'
cargo test -p belfast-experiment-particles --test sky -- --include-ignored
```

```ts
app.setSkyPreset("day"); // "day", "night", "off", or "image" after an upload
app.setSkyImage(rgbaBytes, width, height); // Uint8Array, 2:1; activates the panorama
// Browser helper in web/sky.ts:
const image = await loadSkyPanorama(file);
app.setSkyImage(image.rgba, image.width, image.height);
```

## Gaussian scene

- Standard **binary little-endian** 3DGS PLY, with float `x/y/z`, `f_dc_0..2`, `opacity`, `scale_0..2`, and `rot_0..3`. Property order is arbitrary; additional scalar properties such as normals are skipped. ASCII, compressed PLY, vertex lists, and higher-order SH are rejected explicitly. This courtyard file has DC color only.
- Decode exponential scales, sigmoid opacity, normalized WXYZ rotation, and SH DC color. Transform `(x, y, z)` to `(x, -y, -z)` for the engine's Y-up convention.
- Initial eye is **PLY origin `(0,0,0)`**, looking toward original **+Z** (engine **-Z**). Orbit focus is one unit ahead. FOV is 40°, near/far are 0.02/200 scene units.
- The supplied file has 4,320,000 records; 3,533,087 survive the 1/255 alpha cutoff used by the fragment shader. There is no spatial subsampling.
- Immutable GPU data uses **32 bytes per Gaussian** across two nearest-loaded textures: `RGBA32Float` contains XYZ and covariance normalization; `RGBA32Uint` contains six normalized half-float covariance entries plus RGBA8. Normalization preserves the range of tiny and large scales. The renderer derives covariance once from scale/rotation; source attributes are not re-uploaded each frame.
- The vertex shader projects covariance into a screen-space ellipse, with a 0.3 pixel² minimum filter and a three-sigma extent. Premultiplied alpha blends sorted instances back-to-front, without depth writes.
- A Worker owns a second Wasm instance containing only CPU sort state. Rust culls conservative Gaussian bounds, then performs a stable two-pass radix sort on exact float depth keys. After initialization, only a small camera snapshot and the resulting **u32 indices** cross the Worker boundary; only those indices are uploaded to the GPU.
- Rust checks the camera every frame, but requests a sort only after **0.5° rotation**, **0.02 units translation**, or a projection/viewport change. Thresholds live in `src/sorting.rs` and compare against the last accepted sort, so small movements accumulate. There is at most one request in flight; rendering continues with the previous index order until the next result arrives. A 20% frustum guard band reduces edge popping, but very fast motion can briefly reveal delayed visibility/order updates.
- The scene overlay shows active/visible counts, actual depth-sort count, culling count and Worker processing time. A stationary camera should leave both counts unchanged.

### Compare sorting modes

Use **Sorting → Normal resort / Sort once** without reloading or moving the camera:

- **Normal resort** keeps the existing behavior: cull, then sort visible splats back-to-front when camera motion crosses the update thresholds.
- **Sort once** captures a new depth order for **all** active scene splats at the next update, including offscreen and behind-camera splats. Subsequent updates only cull and compact that fixed order. Newly visible splats therefore still appear; their relative drawing order remains locked to the captured view. Turning or moving away can deliberately expose blending artifacts for comparison.
- Both modes retain the same motion/projection/viewport triggers and the 20% frustum guard band. Culling continues on those updates in both modes, including during camera movement; it is not repeated while the camera is unchanged. One Worker request may be in flight, and drawing continues with the last accepted result.
- Switching modes forces an update even while stationary. Re-entering **Sort once** captures the current camera again. Replies from a previous mode are drained and discarded before new work is sent, so rapid switching cannot overwrite the selected mode or queue a burst of requests.
- **Sort / Cull** counters restart on each switch. In **Sort once**, Sort stays at **1** as Cull increases. The time shows the latest Worker update (including index generation / Wasm-to-JS copy), not GPU frame time or message latency. The first frozen update includes a whole-scene sort and is intentionally more expensive; compare subsequent **Cull only** updates and the FPS meter while moving through similar views. Fixed ordering adds a cached u32 index and visibility mask per active splat, plus retained radix workspace.

The original PLY is about 242 MB with a 113 MB texture payload; the default separated foreground is 177 MB with a 96 MB texture payload; downloading, parsing and sorting need additional CPU/browser memory. Initial decode/upload happens once on the main Rust/Wasm instance; subsequent sorting happens in the Worker. Use a release Wasm build for meaningful performance measurements (`pnpm build:wasm:release`, then refresh the running Vite page).

Format conventions were checked against the [official 3DGS model](https://github.com/graphdeco-inria/gaussian-splatting/blob/main/scene/gaussian_model.py) and the [official covariance projection](https://github.com/graphdeco-inria/diff-gaussian-rasterization/blob/main/cuda_rasterizer/forward.cu). The renderer here is an independent WebGPU implementation.

## Run

From the repository root, with Rust's `wasm32-unknown-unknown` target, `wasm-pack`, Node and pnpm installed:

```sh
pnpm install
pnpm --dir experiments/particles dev
```

Open <http://localhost:5176>. WebGPU requires a supported browser on localhost or HTTPS.

- Left drag: orbit.
- Shift + left drag or middle drag: pan.
- Wheel / trackpad scroll: zoom.

The dev command builds optimized Wasm, starts Vite, and watches `.rs` / `.wgsl` files recursively in `experiments/particles/src` and the shared `crates/belfast/src`. Saving a file rebuilds release Wasm and reloads the page only after a successful build. Saves within 200 ms are combined; edits during a build queue one additional build instead of launching concurrent builds. Errors leave the watcher running so the next save retries. Reloading resets the scene camera and any custom sky image. TypeScript/CSS retain Vite's usual hot reload.

To run only the Rust/WGSL watcher in a separate terminal:

```sh
pnpm --dir experiments/particles watch:wasm
```

Use `dev` for coordinated Vite reloads; `watch:wasm` only regenerates `pkg/`. Both commands build once at startup and stop with Ctrl+C. Generated `pkg`, `dist`, and `target` files are outside the source watch roots, preventing rebuild loops. Cargo manifest changes are not watched; restart the command after changing dependencies. `build:wasm` remains available for a one-time unoptimized debug build, and `build:wasm:release` for a one-time release build. No additional watcher package or Cargo tool is required (Node 20+).

```sh
pnpm --dir experiments/particles build      # release Wasm + typecheck + static site
pnpm --dir experiments/particles preview    # preview dist/
pnpm --dir experiments/particles test
cargo test -p belfast-experiment-particles
cargo test -p belfast-experiment-particles --test gaussian -- --include-ignored # local PLY + GPU readback checks
```

Deploy `dist/` as a static site. Wasm and its JS bindings are bundled into the build; there is no Rust server.

## Access from another computer on the same Wi-Fi

```sh
LAN_HOST=192.168.1.176 pnpm --dir experiments/particles preview:lan
```

Replace `192.168.1.176` with the development computer's current Wi-Fi IP, then open `https://192.168.1.176:5177` on the other computer. This builds the site and serves only `dist/`, bound to the specified IP. With no `LAN_HOST`, preview stays on localhost. Vite's official basic SSL plugin generates a local development certificate cached under ignored `node_modules/`. The usual development server stays on localhost port 5176.

WebGPU requires a secure context, so a plain HTTP LAN address cannot run the renderer. The receiving browser will initially warn about the self-signed HTTPS certificate; handle that prompt yourself after checking the address is your development computer. If the browser still does not expose WebGPU, use a development certificate trusted on that computer. The receiving browser must also support WebGPU.

LAN preview has no hot reload. Rebuild with `pnpm --dir experiments/particles build` after changes and refresh the other computer's browser.

If the connection times out, check that both devices are on the same network, the network allows device-to-device connections, and this computer's firewall permits the Node/Vite server. No router port forwarding is needed.

## Where to work

| File                                       | Responsibility                                                    |
| ------------------------------------------ | ----------------------------------------------------------------- |
| `src/scene.rs`                             | Camera, Gaussian scene, sort scheduling and future simulation.    |
| `src/ply.rs`                               | 3DGS parsing, activation functions and texture packing.           |
| `src/splat.rs`, `src/splat.wgsl`           | Immutable GPU textures, Gaussian projection and alpha blending.   |
| `src/sky.rs`, `src/sky.wgsl`, `web/sky.ts` | Independent background, panorama API and controls.                |
| `src/sorting.rs`                           | Rust frustum culling, radix sort and camera thresholds.           |
| `src/browser.rs`                           | Canvas surface and exported Wasm API; resize and frame lifecycle. |
| `src/input.rs`                             | Owned skeleton snapshot, validation and tracking timeout.         |
| `web/main.ts`                              | Wasm startup, animation scheduling, errors and cleanup.           |
| `web/load-ply.ts`, `web/sort.worker.ts`    | Download progress and Worker lifecycle/protocol.                  |
| `web/input.ts`                             | DOM pointer/wheel events forwarded to Rust in CSS pixels.         |
| `web/kinetic.ts`                           | Optional Kinetic-compatible adapter; no runtime dependency.       |
| `pkg/`                                     | Generated wasm-pack output. Do not edit or commit.                |

Rust owns every GPU resource, all camera calculations and all rendering. JavaScript calls one `frame()` per animation frame; it never fetches particle data or camera matrices.

## Public API

The host loads one model per `ExperimentApp`/Worker pair. To replace a model, stop the old RAF and Worker and create a new pair; swapping models while a sort is in flight is not supported by this protocol.

```ts
await init();
const app = await ExperimentApp.create(canvas);
app.resize(cssWidth, cssHeight, devicePixelRatio);
app.loadPly(plyBytes); // Uint8Array; returns active Gaussian count
const positions = app.takeSortPositions(); // transfer once to GaussianSorter in Worker
const camera = app.requestSort(); // empty if unchanged or a sort is already in flight
// Worker: new GaussianSorter(positions).sort(camera) → Uint32Array
app.applySort(sortedIndices, didSort); // didSort=false for a culling-only result; omitted defaults to true
app.frame(performance.now());
app.pointerDown(x, y, button, shiftKey); // button 0 = primary, 1 = middle
app.pointerMove(x, y, cssWidth, cssHeight);
app.pointerUp(); // also use on cancel / blur
app.scroll(deltaInPixels);
app.setSkeleton(points, performance.now());
app.clearSkeleton();
app.free(); // stop RAF and input listeners first
```

The Worker calls `sorter.setFrozen(true)` once per activation, then continues calling `sorter.sort(camera)` for current visibility. `sorter.didSort()` distinguishes depth-sort work from culling-only updates. On a mode change the host calls `app.invalidateSort()` and tags Worker messages with a revision. A stale reply is acknowledged with `app.discardSort()` instead of `applySort()`, freeing the pending request without accepting its old camera or indices.

`setSkeleton` copies a `Float32Array` with exactly 132 values: 33 MediaPipe landmarks in Kinetic's fixed order, each `[x, y, z, confidence]`. Coordinates must be finite; confidence must be in `[0, 1]`. Timestamps must be finite, non-negative, non-decreasing milliseconds from the same `performance.now()` clock as `frame()`. Malformed input throws without replacing the previous accepted frame. `clearSkeleton()` removes the pose immediately; `SkeletonInput.latest(now_ms)` returns no pose after 250 ms without an update.

The experiment currently only stores the pose. Read `self.skeleton.latest(now_ms)` in `Scene::update` when adding forces, emitters, or a GPU skeleton buffer.

## Connect Kinetic later

Add your local Kinetic package when needed, for example from this folder:

```sh
pnpm add kinetic@file:../../../kinetic
```

Inside `start()`, after `ExperimentApp.create(canvas)` and the existing `createSkeletonAdapter(created)` call, the wiring is:

```ts
// Add at the top of web/main.ts after installing the dependency:
import { SkeletonManager } from "kinetic";

// In start():
const tracker = new SkeletonManager({ mirror: true });
tracker.addEventListener("skeleton-detected", ({ detail }) => {
  skeleton.update(detail.poses, performance.now());
});
tracker.addEventListener("error", () => skeleton.clear());
await tracker.init();
```

Keep the tracker in the host's lifecycle state, and call `tracker.dispose()` before `app.free()` in `dispose()`. Also dispose the tracker if initialization finishes after the host has been disposed, matching the existing Wasm initialization guard. Start camera initialization from the user interaction appropriate to your experiment.

The adapter takes the first pose's `keypoints3D`, preserves Kinetic's already-mirrored X, and maps `[x, y, z]` to `[x, -y, -z]`: Y up and positive Z toward the initial viewer. One metre becomes one scene unit. Missing confidence becomes zero; empty/malformed poses clear tracking. Rust owns a copy, so the adapter reuses its packing buffer.

These are estimated hip-relative coordinates, not a calibrated position in the room. If the whole body should move across the scene, combine image-space hip position with the local 3D skeleton in this adapter. Kinetic currently performs synchronous inference on the main thread; moving it into a Worker is a separate optimization if profiling shows dropped frames.

## Template

Use [`../template`](../template/README.md) to start another experiment. This folder owns its own source, generated Wasm and build output; changes here do not affect the original template.
