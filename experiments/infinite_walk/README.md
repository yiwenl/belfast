# Infinite walk

Copied from `experiments/template`: a Rust-owned WebGPU scene with a looping forest point cloud and a wheel-controlled camera.

## Run

```sh
pnpm --dir experiments/infinite_walk dev
```

Open <http://localhost:5186>. Scroll down to walk forward; scroll up to walk backward. Movement eases to a stop and loops in both directions. The camera follows the original birdsong path at an adjustable height and fixed heading. A mouse wheel or trackpad is required; pinch gestures retain browser zoom.

**Resolution** defaults to **HD · 1920 × 1080**, with an exact 1920 × 1080 drawing buffer regardless of device pixel ratio. CSS centers and fits the image inside the browser at 16:9. Select **Full browser** to fill the viewport and render at its size multiplied by device pixel ratio.

The Tweakpane panel shows FPS, a live **Particle size** slider (0.001–0.05 world units, default 0.01), and **Camera Y** (0–3 world units, default 0.59). **Camera pitch (°)** tilts the view from −85° (down) to +85° (up), with 0° level and +4° as the default. Scrolling follows the horizontal walking path at every pitch.

Enable **White particles** to use a white base color; disable it to restore the original point-cloud colors (the default). Both modes respond to the spotlight.

**FOV (°)** adjusts the vertical field of view from 10° to 120° (default 88°).

The **Spotlight** follows the camera, defaulting to 2.9 units ahead and 4.2 units above it, intensity 30, range 5.5 and FOV 33°, and shines straight down along world −Y. Its folder controls **Forward**, **Height** (relative to the camera), **Intensity**, **Range** and **FOV (°)**. Spotlight FOV is the full cone angle (10°–150°), with a soft edge starting at two-thirds of the outer half-angle. Point brightness uses this cone, distance falloff and dim ambient illumination, in linear color space. Lighting uses each point's wrapped position so it stays aligned across walking loops, and applies to both original and white colors.

Particles cast circular silhouettes into a 2048² perspective shadow map aligned with the spotlight's position, FOV and range. A fixed 5×5 tent-weighted PCF kernel with bilinear comparison sampling softens the shadows and reduces moving-light shimmer. **Shadows** controls their strength (0–1, default 0.85); zero disables shadowing. Shadows reduce direct light while preserving ambient illumination. Particle silhouettes face the light in the depth pass and receive shadows at their centers; this approximates solid particles without adding surface geometry. Spatial filtering does not provide temporal antialiasing, so very fine silhouettes can still shimmer in motion. Bloom is not included.

**Show shadow map** toggles a live square preview in the bottom-left corner (enabled by default). It displays the actual shadow texture as linearized depth: near is dark, far or empty is white. The map continues updating while the preview is visible, even if shadow strength or light intensity is zero.

## Drifting particles

The **Count** control adjusts the number of drifting billboard particles from 0 to 500,000 (default 119,565). Increasing the count spawns additional particles in the current spotlight. Simulation and both draw passes use the active count. Each frame, a compute pass compacts the current spotlight's point-cloud positions into a spawn list. New particles and particles whose life has ended choose a random entry, inheriting its color. The background cloud is read-only. Empty light volumes leave expired particles hidden until a valid source enters the light; existing particles finish their lives. Lifetime defaults to 5 seconds with ±40% variation. **Outside decay (×)** controls how quickly particles age outside the spotlight cone or beyond its range (1–10×, default 2×). Inside the light they age at normal speed. Particle size fades in and out with age.

`belfast::filament::FilamentSimulation` drives the vortex rings. **Filament → Ring count** adjusts the number of active rings from 1 to 24 (default 12), with 12 vertices per ring. Changing the count reseeds the rings while existing particles keep their positions and ages. Rings simulate in local coordinates; their world origin follows the spotlight's XZ center at **Ground Y** (default −0.11, based on this cloud's floor). This avoids reseeding on every camera movement. A compute pass advects the particles with filament velocity, air drag and upward drift. Camera loop rebasing preserves particle ages. The same updated particle buffer feeds both the shadow and visible passes: particles cast shadows onto the forest and each other, and receive the shared 5×5 PCF shadows.

**Drifting particles** provides visibility, animation, size, lifetime, lifetime variation, particle updraft and drag. **Filament** provides ground height, ring updraft, XZ drift, ring/core radius, vortex strength, circulation and ring lifetime. Enable **Debug view** to draw colored filament rings over the scene. Ground height is an adjustable plane rather than terrain collision; no collision solver is included.

## Assets

`public/forest.ply` is a local copy of `forest_walk_patched_final.ply` from `/Users/yi-wenlin/Development/Sketches/experiments2/apps/birdsong/public/` (370,681 points). The source is already rotated and patched; positions and RGB remain unchanged. PLY assets are ignored by this repository. On another checkout, copy that file to `public/forest.ply` before starting or building.

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
