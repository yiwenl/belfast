# Forest Walk point cloud design

Approved in chat: retain the PLY RGB colors and smoothly move between adjacent recorded cameras.

- Input: forest_walk_2_dense.ply (2,132,926 vertices) and forest_walk_2_cameras.json (200 poses), from E:/pointClouds/forest_walk_2/output.
- Render opaque, unlit camera-facing quads. Discard fragments with length(point_coord) > 0.5. Preserve the recorded RGB, use depth testing/writing, and expose a point diameter slider in CSS pixels.
- Scroll down selects the next recorded camera; scroll up selects the previous. Clamp at either end. Interpolate position and orientation along the recorded sequence, including when repeated scroll input changes the destination.
- Start at the first recorded frame. Use the recorded orientation (including roll) and undistorted vertical field of view. Adapt the horizontal field of view to the viewport.
- Reuse the Rust/wgpu/Wasm scene and Vite host in experiments/forest_walk. Keep coordinates in the shared reconstruction world and convert camera axes from OpenCV right/down/forward to renderer right/up/back.
- Copy local assets into ignored public files through a repeatable preparation command. Export only camera data needed by the viewer.
- Display loading progress, errors, point count and current camera number.

## Approaches

User selected smooth adjacent camera movement over instant jumps, and original RGB over a uniform cloud color. Retain the existing Rust rendering architecture; an independent JavaScript rendering engine would duplicate the template.

## Implementation choices

Use instanced vertex buffers (16 bytes per point), a depth attachment, and screen-space billboard expansion. Use accumulated wheel distance for trackpads, with at most one selected camera per wheel event. Ignore pinch zoom and horizontal-only input. Interpolate along camera indices, then interpolate the two surrounding poses.
