# Infinite walk

Copied from `experiments/template`: a Rust-owned WebGPU scene with a looping forest point cloud and a wheel-controlled camera.

## Forest atmosphere

The outer forest uses deep teal distance haze and grouped ochre, grey lilac, moss and dusty cyan colors. Three repeating depth layers surround the original walking corridor. The original ground receives a subtle height-blended tint at load time, which also carries into particles spawned from it. White mode still overrides point colors. Indexed quads reuse four vertices per circle, including both original cloud copies and particle shadows.

`pnpm prepare:backdrop` converts the local `../firefly_forest/public/forest.ply` Gaussian asset into `public/backdrop.ply`: an 80,000-point maximum, deterministic, spatially thinned RGB cloud (~1.28 MB). Both PLY assets are local and gitignored. `dev` and `build` prepare the backdrop automatically; keep the source asset available. The backdrop has its own inexpensive vertex pass and does not enlarge simulation or shadow buffers. It follows the same 13.3-unit loop and lateral drift as the path.

The backdrop conversion uses `y = -sourceY + 0.5`, aligning its ground band with the original forest near `Y = -0.11`. Its lower crop is `Y = -0.15` so ground points and trunk bases remain visible; a higher crop removes these supports and makes the outer forest appear suspended.

**Scene → PLY point cloud → Backdrop Y offset** shifts the backdrop vertically in world units (-3 to 3, default 0). Negative values lower it and positive values raise it, across all three repeating depth layers.

**Backdrop particle size** sets the backdrop billboards' world-space diameter (0.001–0.25, default 0.065), independently of the original cloud's particle size.

## Performance checks

Open `http://localhost:5186/?benchmark=static` for a 5-second warmup plus 30-second stationary measurement, or `?benchmark=walk` for 85 seconds of automatic walking after warmup (longer than the 80-second lap). Both use the default exact 1920×1080 canvas and disable settings and interaction while measuring. The visible JSON counts GPU completion callbacks over elapsed time, including batched completions and stalls, and invalidates a run if the page becomes hidden. `p95CompletionBatchMs` describes gaps between observed completion batches, not per-frame GPU latency. Keep other GPU work idle. Browser FPS is limited by presentation/refresh scheduling; report raw values without rounding a sub-60 result into a pass. Earlier submission-only baseline files are explicitly excluded from acceptance evidence.

For a separate, uncapped native throughput diagnostic that waits for GPU completion after every frame, run `cargo test -p belfast-experiment-infinite-walk --test performance -- --ignored --nocapture`. It uses 1920×1080 rgba16float, the full particle simulation, shadows, fog and backdrop. This diagnoses GPU headroom; it is not a substitute for browser FPS.

## Run

```sh
pnpm --dir experiments/infinite_walk dev
```

Open <http://localhost:5186>. Scroll down to walk forward or up to walk backward. Scroll movement eases to a stop and loops in both directions. Press **P** to walk backward automatically at one lap per 80 seconds. Press it again to stop in place. Press **F** to toggle fullscreen for the canvas alone. The camera follows the original birdsong path at an adjustable height and fixed heading. Pinch gestures retain browser zoom.

**Resolution** defaults to **HD · 1920 × 1080**. Select **Portrait · 1080 × 1920** for a vertical 9:16 canvas. Both presets use their exact pixel dimensions and fit the browser window at their native aspect ratio. **Full browser** fills the viewport and renders at its size multiplied by device pixel ratio.

**Scene → Display → Output** shows the selected **HDR** or **SDR** canvas mode. HDR is requested automatically through Belfast's surface selection: supported browsers use `rgba16float` with extended sRGB and `toneMapping.mode: "extended"`, falling back to SDR when unavailable. The scene and fog targets share this floating-point format, preserving values above SDR white through particle lighting and fog composition. The shaders calculate lighting in linear space and encode extended sRGB without clamping highlights. Actual highlight brightness depends on the display and OS settings; see [WebGPU HDR output](https://developer.chrome.com/blog/new-in-webgpu-129#hdr-support-with-canvas-tone-mapping-mode).

**Light → Cylinder light → Intensity** ranges from **0 to 300** (default 100). Increase it to emphasize HDR highlights in both the forest and drifting particles, along with the shadowed volumetric fog.

Press **Ctrl+S** or **⌘S** to download the current canvas frame as a PNG at its rendering resolution.

Move the mouse over the canvas to interact at the ray's intersection with an invisible, camera-facing plane through the midpoint of the light cylinder. **Particles → Pointer fluid → Show pointer ball** displays a red sphere at that point (off by default); hiding it keeps the fluid and particle interaction active. The plane follows the light and camera each frame. The marker respects scene depth and fog, and disappears when the pointer leaves the canvas or enters fullscreen letterboxing. Pointer mapping follows the displayed canvas area in every resolution mode.

**Particles → Pointer fluid** lets the moving red sphere drive a 128² `belfast::fluid` grid, whose world-XY velocity then moves nearby drifting particles. The grid stays centered on the light rather than following the pointer and covers 12 × 12 world units. **Radius** sets the injection brush radius and Z falloff distance from the light-center plane (0.05–3, default 0.66). **Strength** scales the red sphere's XY displacement into a fluid impulse (0–10, default 7.2); zero disables and clears the interaction. Splats cover the pointer's path so direction and travel distance drive the flow. A stationary pointer injects nothing, and the field continues to advect and dissipate after movement stops or the pointer leaves. Re-entry and resuming from pause establish a fresh position without a jump impulse. Particles sample the persistent field, including trails behind the pointer, with no direct repulsion or collision against the sphere. Smooth animated noise adds smaller positive and negative Z motion proportional to the sampled fluid speed, so a still field adds no motion. Interaction remains independent of the filament force multiplier. Pausing particles also pauses the fluid. The original PLY cloud stays fixed, and displaced drifting particles continue casting and receiving shadows.

FPS stays visible above the **Scene**, **Light**, **Particles** and **Filament** tabs. The **Scene** tab includes a live **Particle size** slider (0.001–0.05 world units, default 0.01), and **Camera Y** (0–3 world units, default 1.24). **Camera pitch (°)** tilts the view from −85° (down) to +85° (up), with 0° level and −13° as the default. Scrolling follows the horizontal walking path at every pitch.

Enable **White particles** to use a white base color; disable it to restore the original point-cloud colors (the default). Both modes respond to the cylinder light.

**FOV (°)** adjusts the vertical field of view from 10° to 120° (default 88°).

The **Cylinder light** shines from the camera’s upper right, tilted 15° from vertical toward the middle of the path. Its axis meets the forest floor 2.9 units ahead of the camera, and its source sits 4.2 units above camera height. **Radius** sets the beam radius perpendicular to its axis (0.05–5, default 1.45). **Range** sets the distance along the axis (default 7.6). Light fades across the outer third of the radius and with depth. **Forward**, **Height** and **Intensity** (default 100) are adjustable. Lighting uses each point's wrapped position so it stays aligned across walking loops, and applies to both original and white colors.

Particles cast circular silhouettes into a 2048² orthographic shadow map covering the cylinder’s cross section and axial range. A fixed 5×5 tent-weighted PCF kernel, reconstructed with nine phase-aware bilinear comparison samples, softens the shadows and reduces moving-light shimmer. **Light → Shadows → Strength** controls their strength (0–1, default 0.97); zero disables shadowing. Shadows attenuate both direct and ambient light inside the cylinder: at strength 1, fully occluded particle surfaces are black. PCF transition regions can remain partially lit, and illuminated fog in front of a dark surface can still contribute brightness. Particle silhouettes face the light in the depth pass and receive shadows at their centers; this approximates solid particles without adding surface geometry. Spatial filtering does not provide temporal antialiasing, so very fine silhouettes can still shimmer in motion. Bloom is not included.

**Show shadow map** toggles a live square preview in the bottom-left corner (hidden by default). It displays the actual shadow texture as linear depth, with dark pixels near the light and white pixels at the far end or outside the beam. The map continues updating while the preview is visible, even if shadow strength or light intensity is zero.

**Light → Volumetric fog → Density** controls the ground-level density inside the light cylinder (default 0.22 per world unit). Zero disables the fog pass. The renderer marches each camera ray up to the nearest visible surface and samples the current shadow map along the way. Forest points and drifting particles block the light that reaches the mist. **Shadows → Strength** controls how strongly that occlusion affects both the fog and the particles.

**Ground half-height** sets the distance over which the ground layer loses half its density (default 0.45 world units). The layer peaks at the forest floor (world Y = −0.11) and decays exponentially with distance from that plane. Lower the half-height for a thinner layer, or raise it to let mist fill more of the path.

Smooth 3D noise varies the local fog density between 65% and 135% and drifts slowly with time. **Light → Volumetric fog → Noise frequency** sets 2–64 noise cells per walking loop (default 16). Lower values produce larger patches; higher values produce finer detail. Whole-cell periods preserve continuity when the camera loops, and adjusting frequency preserves the wind's world-space speed.

## Drifting particles

The **Count** control adjusts the number of drifting billboard particles from 0 to 500,000 (default 250,000). Increasing the count spawns additional particles inside the light cylinder. Simulation and both draw passes use the active count. Each frame, a compute pass compacts the point-cloud positions inside the cylinder into a spawn list. New particles and particles whose life has ended choose a random entry, inheriting its color. The background cloud is read-only. Empty light volumes leave expired particles hidden until a valid source enters the light; existing particles finish their lives. Lifetime defaults to 4.5 seconds with ±45% variation. **Outside decay (×)** controls how quickly particles age outside the cylinder (1–10×, default 3.8×). Inside the light they age at normal speed. Particle size fades in and out with age.

`belfast::filament::FilamentSimulation` drives the vortex rings. **Filament → Ring count** adjusts the number of active rings from 1 to 24 (default 6), with 12 vertices per ring. Changing the count reseeds the rings while existing particles keep their positions and ages. Rings simulate in local coordinates; their world origin follows the cylinder’s XZ center at **Ground Y** (default −0.11, based on this cloud's floor). **Filament → Rings → Interval reset** (off by default) restarts all active rings together while **Animate** is on. **Reset interval (s)** sets the base delay from 0 to 20 seconds (default 3.9); each countdown adds a fresh random 0–1 seconds, giving 3.9–4.9 seconds at the default. Changing the base delay starts a new countdown. Switching Interval reset off pauses that countdown; switching it on resumes it. Individual rings still renew when their lifetimes end. Rings keep their current state and interval countdown when the camera stops scrolling. Drifting particles keep their positions and ages. A compute pass advects the particles with filament velocity, air drag and upward drift. Camera loop rebasing preserves particle ages. The same updated particle buffer feeds both the shadow and visible passes: particles cast shadows onto the forest and each other, and receive the shared 5×5 PCF shadows.

**Particles → Motion → Force multiplier** scales the filament flow and particle updraft together from 0 to 2 (default 0.98). **Gravity** applies continuous downward acceleration from 0 to 2 world units/s² (default 0.43), independently of the flow multiplier and each particle's random force scale. Particles follow the current filament velocity directly each frame, as in vortexspheres' normal advection mode; they do not retain upward or sideways momentum from previous flow. Gravity-driven velocity is stored separately, with **Fall drag** controlling its damping (default 3.6). Changing Fall drag does not smooth or strengthen the filament response. As the current flow weakens, the independent falling motion takes over. At zero force multiplier particles still fall; Gravity zero stops adding downward acceleration and any existing falling velocity decays with Fall drag. The filament field still fades with distance rather than ending at the debug ring lines. **Filament** provides ground height, ring updraft, XZ drift, ring/core radius, vortex strength, circulation and ring lifetime. Enable **Debug view** to draw colored filament rings over the scene. **Ground Y** also sets a flat collision floor: new and existing particles stay at least half their full diameter above it. Contact removes downward velocity while preserving horizontal motion and upward flow, so filaments can lift resting particles again. This plane does not follow the point-cloud terrain.

## Assets

`public/forest.ply` is a local copy of `forest_walk_patched_final.ply` from `/Users/yi-wenlin/Development/Sketches/experiments2/apps/birdsong/public/` (370,681 points). Each original point has one same-color copy. **Copy offset** sets the maximum displacement per axis (0–0.25 world units, default 0.011). Each copy follows its fixed random direction as the slider moves and overlaps its original at zero. Both sets share the rendering, shadow and spawn passes, for 741,362 forest points. PLY assets are ignored by this repository. On another checkout, copy that file to `public/forest.ply` before starting or building.

The camera uses birdsong's start `[-0.3, 0.3, 4.3]`, end `[0.1, 0.3, -9]`, an 88° vertical field of view, and 0.0005 path cycles per wheel pixel. Points repeat along that same diagonal in the vertex shader. Each lap rebases the camera and cloud together to retain precision. Points render as opaque, depth-tested circular billboards with a diameter of 0.01 world units, so they appear larger near the camera.

## Work here

- `src/walk.rs`: scroll speed, damping and looping camera position.
- `src/points.wgsl`: point-cloud wrapping and billboard rendering.
- `src/scene.rs`: GPU buffers, camera and render pass.
- `src/point_cloud.rs`: binary PLY loader reused from `forest_walk`.
- `src/particles.rs` and `src/particle-*.wgsl`: GPU spawn selection, independent particle state and filament-driven motion.
- `web/main.ts`: asset loading and the template's Wasm lifecycle.

The template's optional skeleton input remains available for later particle interaction. Spray textures are not included.

## Verify

```sh
cargo test -p belfast-experiment-infinite-walk
cargo test -p belfast-experiment-infinite-walk --test render -- --ignored
pnpm --dir experiments/infinite_walk test
pnpm --dir experiments/infinite_walk build
pnpm --dir experiments/infinite_walk preview
```

The render test needs a GPU and the local PLY asset. Preview uses <http://localhost:4186>. `dev` watches Rust/WGSL and rebuilds Wasm, while Vite updates TypeScript/CSS. The build includes the local PLY in `dist/`.
