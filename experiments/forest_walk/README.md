# Forest Walk

An opaque RGB point cloud viewed from the camera poses recorded during the forest walk. Rust/wgpu renders instanced circular billboards with depth testing. Each dot retains its PLY color.

## Run

Requires Node 20+, pnpm, Rust with the wasm32-unknown-unknown target, and wasm-pack.

From the repository root:

```sh
pnpm install
pnpm --dir experiments/forest_walk prepare:assets -- "E:/pointClouds/forest_walk_2/output"
pnpm --dir experiments/forest_walk dev
```

Open http://localhost:5182.

- Scroll down to select the next recorded camera; scroll up for the previous camera.
- Arrow keys also step through cameras. Scrolling and keys clamp at both ends.
- Drag with the left mouse button (or one finger) to look around in place. Vertical rotation stops before the poles to prevent flipping.
- Scrolling smoothly follows the recorded positions while keeping your chosen viewing direction and field of view. The initial view uses camera 1, including its recorded roll.
- Point size controls the circular billboard diameter in CSS pixels (0.5–12 px). Default: 2 px.
- Camera displays the selected destination frame. The walk ends at camera 151 (inclusive); tracking from camera 152 onward is unreliable.

The preparation command copies the PLY to public/forest.ply and writes public/cameras.json with the first 151 poses in timestamp order and the intrinsics required by the viewer. These local assets are ignored by Git. Repeat this step on another checkout or before building for deployment.

## Build and verify

```sh
pnpm --dir experiments/forest_walk test
cargo test -p belfast-experiment-forest-walk
cargo test -p belfast-experiment-forest-walk --test real_assets -- --ignored
pnpm --dir experiments/forest_walk build
pnpm --dir experiments/forest_walk preview
```

The build bundles Wasm and copies the prepared assets into dist/. Preview uses http://localhost:4177.

## Implementation

- src/point_cloud.rs: binary little-endian PLY decoding; keeps original positions and RGB.
- src/camera_path.rs: camera JSON validation, OpenCV camera-axis conversion, path interpolation and endpoint clamping.
- src/points.wgsl: screen-space billboard expansion and length(point_coord) > 0.5 circle discard.
- src/scene.rs: immutable instanced vertex buffer, depth attachment and camera uniforms.
- web/main.ts: asset loading, lifecycle, controls and status.
- web/wheel.ts: line/page/pixel wheel normalization and trackpad accumulation.

The point cloud and cameras stay in the same reconstruction world. Camera matrices are row-major OpenCV right/down/forward and become renderer right/up/back without changing the world coordinates. The undistorted vertical field of view is preserved; the horizontal field of view adapts to the viewport. Distances are in reconstruction units, not metres.

## Local point-cloud cleanup

The supplied cloud now contains 2,011,687 points. The detached tail (121,239 points supported only by cameras 152–200 in COLMAP fusion visibility) was removed. Retained point positions, normals and RGB values are unchanged. The original PLY, visibility file and camera JSON are backed up under E:/pointClouds/forest_walk_2/output/backups/before-tail-cleanup-20261025-203901. The cleanup report is E:/pointClouds/forest_walk_2/output/forest_walk_2_tail_cleanup.json.
