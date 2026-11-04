# Infinite walk

Copied from `experiments/template`: a Rust-owned WebGPU scene with a looping forest point cloud and a wheel-controlled camera.

## Run

```sh
pnpm --dir experiments/infinite_walk dev
```

Open <http://localhost:5186>. Scroll down to walk forward; scroll up to walk backward. Movement eases to a stop and loops in both directions. The camera follows the original birdsong path at an adjustable height and fixed heading. A mouse wheel or trackpad is required; pinch gestures retain browser zoom.

**Resolution** defaults to **HD · 1920 × 1080**. Select **Portrait · 1080 × 1920** for a vertical 9:16 canvas. Both presets use their exact pixel dimensions and fit the browser window at their native aspect ratio. **Full browser** fills the viewport and renders at its size multiplied by device pixel ratio.

FPS stays visible above the **Scene**, **Light**, **Particles** and **Filament** tabs. The **Scene** tab includes a live **Particle size** slider (0.001–0.05 world units, default 0.01), and **Camera Y** (0–3 world units, default 0.75). **Camera pitch (°)** tilts the view from −85° (down) to +85° (up), with 0° level and +4° as the default. Scrolling follows the horizontal walking path at every pitch.

Enable **White particles** to use a white base color; disable it to restore the original point-cloud colors (the default). Both modes respond to the cylinder light.

**FOV (°)** adjusts the vertical field of view from 10° to 120° (default 88°).

The **Cylinder light** shines straight down from 2.9 units ahead of the camera and 4.2 units above it. **Radius** sets the beam radius in world units (0.05–5, default 1.45) and **Range** sets its vertical reach (default 7.6). The footprint has the same radius at every depth. Light fades across the outer third of the radius and with depth. **Forward**, **Height** and **Intensity** (default 30) are adjustable. Lighting uses each point's wrapped position so it stays aligned across walking loops, and applies to both original and white colors.

Particles cast circular silhouettes into a 2048² orthographic shadow map covering the cylinder’s circular footprint and vertical range. A fixed 5×5 tent-weighted PCF kernel with bilinear comparison sampling softens the shadows and reduces moving-light shimmer. **Light → Shadows → Strength** controls their strength (0–1, default 0.90); zero disables shadowing. Shadows reduce direct light while preserving ambient illumination. Particle silhouettes face the light in the depth pass and receive shadows at their centers; this approximates solid particles without adding surface geometry. Spatial filtering does not provide temporal antialiasing, so very fine silhouettes can still shimmer in motion. Bloom is not included.

**Show shadow map** toggles a live square preview in the bottom-left corner (enabled by default). It displays the actual shadow texture as linear depth, with dark pixels near the light and white pixels at the far end or outside the beam. The map continues updating while the preview is visible, even if shadow strength or light intensity is zero.

## Drifting particles

The **Count** control adjusts the number of drifting billboard particles from 0 to 500,000 (default 250,000). Increasing the count spawns additional particles inside the light cylinder. Simulation and both draw passes use the active count. Each frame, a compute pass compacts the point-cloud positions inside the cylinder into a spawn list. New particles and particles whose life has ended choose a random entry, inheriting its color. The background cloud is read-only. Empty light volumes leave expired particles hidden until a valid source enters the light; existing particles finish their lives. Lifetime defaults to 4.5 seconds with ±45% variation. **Outside decay (×)** controls how quickly particles age outside the cylinder (1–10×, default 3.8×). Inside the light they age at normal speed. Particle size fades in and out with age.

`belfast::filament::FilamentSimulation` drives the vortex rings. **Filament → Ring count** adjusts the number of active rings from 1 to 24 (default 12), with 12 vertices per ring. Changing the count reseeds the rings while existing particles keep their positions and ages. Rings simulate in local coordinates; their world origin follows the cylinder’s XZ center at **Ground Y** (default −0.11, based on this cloud's floor). When the camera finishes easing after a wheel or trackpad scroll, all active rings restart once at the new location with the current filament settings. Drifting particles keep their positions and ages. A compute pass advects the particles with filament velocity, air drag and upward drift. Camera loop rebasing preserves particle ages. The same updated particle buffer feeds both the shadow and visible passes: particles cast shadows onto the forest and each other, and receive the shared 5×5 PCF shadows.

**Particles → Motion → Force multiplier** scales the filament flow and particle updraft together from 0 to 2 (default 0.98). At zero, existing velocity decays with drag. **Filament** provides ground height, ring updraft, XZ drift, ring/core radius, vortex strength, circulation and ring lifetime. Enable **Debug view** to draw colored filament rings over the scene. Ground height is an adjustable plane rather than terrain collision; no collision solver is included.

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
