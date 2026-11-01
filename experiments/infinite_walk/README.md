# Infinite walk

Copied from `experiments/template`: a Rust-owned WebGPU scene with a looping forest point cloud and a wheel-controlled camera.

## Run

```sh
pnpm --dir experiments/infinite_walk dev
```

Open <http://localhost:5186>. Scroll down to walk forward; scroll up to walk backward. Movement eases to a stop and loops in both directions. The camera follows the original birdsong path at an adjustable height and fixed heading. A mouse wheel or trackpad is required; pinch gestures retain browser zoom.

The Tweakpane panel shows FPS, a live **Particle size** slider (0.001–0.05 world units, default 0.01), and **Camera Y** (−2–3 world units, default 0.3). Height changes move the eye and look-at target together, preserving the camera's heading.

Enable **White particles** to use a white base color; disable it to restore the original point-cloud colors (the default). Both modes respond to the spotlight.

**FOV (°)** adjusts the vertical field of view from 10° to 120° (default 45°).

The **Spotlight** follows the camera, defaulting to 3.5 units ahead and 2 units above it with a range of 3.3 units, and shines straight down along world −Y. Its folder controls **Forward**, **Height** (relative to the camera), **Intensity** and **Range**. Point brightness uses a soft cone (30° inner / 45° outer half-angle), distance falloff and dim ambient illumination, in linear color space. Lighting uses each point's wrapped position so it stays aligned across walking loops, and applies to both original and white colors. This first lighting pass has no shadows or bloom.

## Assets

`public/forest.ply` is a local copy of `forest_walk_patched_final.ply` from `/Users/yi-wenlin/Development/Sketches/experiments2/apps/birdsong/public/` (370,681 points). The source is already rotated and patched; positions and RGB remain unchanged. PLY assets are ignored by this repository. On another checkout, copy that file to `public/forest.ply` before starting or building.

The camera uses birdsong's start `[-0.3, 0.3, 4.3]`, end `[0.1, 0.3, -9]`, 45° vertical field of view, and 0.0005 path cycles per wheel pixel. Points repeat along that same diagonal in the vertex shader. Each lap rebases the camera and cloud together to retain precision. Points render as opaque, depth-tested circular billboards with a diameter of 0.01 world units, so they appear larger near the camera.

## Work here

- `src/walk.rs`: scroll speed, damping and looping camera position.
- `src/points.wgsl`: point-cloud wrapping and billboard rendering.
- `src/scene.rs`: GPU buffers, camera and render pass.
- `src/point_cloud.rs`: binary PLY loader reused from `forest_walk`.
- `web/main.ts`: asset loading and the template's Wasm lifecycle.

The template's optional skeleton input remains available for later particle interaction. This experiment contains the static point cloud, walking camera and following spotlight; particle simulation and spray textures are not added yet.

## Verify

```sh
cargo test -p belfast-experiment-infinite-walk
cargo test -p belfast-experiment-infinite-walk --test render -- --ignored
pnpm --dir experiments/infinite_walk test
pnpm --dir experiments/infinite_walk build
pnpm --dir experiments/infinite_walk preview
```

The render test needs a GPU and the local PLY asset. Preview uses <http://localhost:4186>. `dev` watches Rust/WGSL and rebuilds Wasm, while Vite updates TypeScript/CSS. The build includes the local PLY in `dist/`.
