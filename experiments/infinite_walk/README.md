# Infinite walk

Copied from `experiments/template`: a Rust-owned WebGPU scene with a looping forest point cloud and a wheel-controlled camera.

## Run

```sh
pnpm --dir experiments/infinite_walk dev
```

Open <http://localhost:5186>. Scroll down to walk forward; scroll up to walk backward. Movement eases to a stop and loops in both directions. The camera follows the original birdsong path at a fixed height and heading. A mouse wheel or trackpad is required; pinch gestures retain browser zoom.

## Assets

`public/forest.ply` is a local copy of `forest_walk_patched_final.ply` from `/Users/yi-wenlin/Development/Sketches/experiments2/apps/birdsong/public/` (370,681 points). The source is already rotated and patched; positions and RGB remain unchanged. PLY assets are ignored by this repository. On another checkout, copy that file to `public/forest.ply` before starting or building.

The camera uses birdsong's start `[-0.3, 0.3, 4.3]`, end `[0.1, 0.3, -9]`, 45° vertical field of view, and 0.0005 path cycles per wheel pixel. Points repeat along that same diagonal in the vertex shader. Each lap rebases the camera and cloud together to retain precision. Points render as opaque, depth-tested circles, 2 CSS pixels in diameter.

## Work here

- `src/walk.rs`: scroll speed, damping and looping camera position.
- `src/points.wgsl`: point-cloud wrapping and billboard rendering.
- `src/scene.rs`: GPU buffers, camera and render pass.
- `src/point_cloud.rs`: binary PLY loader reused from `forest_walk`.
- `web/main.ts`: asset loading and the template's Wasm lifecycle.

The template's optional skeleton input remains available for later particle interaction. This initial experiment contains the static point cloud and walking camera; particle simulation, spray textures and lighting are not added yet.

## Verify

```sh
cargo test -p belfast-experiment-infinite-walk
cargo test -p belfast-experiment-infinite-walk --test render -- --ignored
pnpm --dir experiments/infinite_walk test
pnpm --dir experiments/infinite_walk build
pnpm --dir experiments/infinite_walk preview
```

The render test needs a GPU and the local PLY asset. Preview uses <http://localhost:4186>. `dev` watches Rust/WGSL and rebuilds Wasm, while Vite updates TypeScript/CSS. The build includes the local PLY in `dist/`.
