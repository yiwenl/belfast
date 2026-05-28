# Camera

Base camera with view and projection matrices (CPU-side math via `gl-matrix`, no GPU resources).

## Import

```ts
import { Camera } from "belfast";
import type { Vec3, Mat4 } from "belfast";
```

`Vec3` is now backed by `gl-matrix` `ReadonlyVec3`, and `Mat4` is `Float32Array` matrix data compatible with `gl-matrix` `mat4` operations.

## Methods

### `lookAt(eye, target, up?)`

Builds the view matrix from eye position, look-at target, and up vector (default `[0, 1, 0]`). Stores values used by `getPosition()` and `getLookAtTarget()`.

### `getViewMatrix()`

Column-major `Float32Array` (16 elements), WGSL-compatible.

### `getProjectionMatrix()`

Projection matrix from the subclass (`PerspectiveCamera` or `OrthographicCamera`).

### `getViewProjectionMatrix(out?)`

Returns `projection * view` for `clip = viewProj * vec4(position, 1.0)`. Reuses an internal buffer when `out` is omitted.

### `Camera.uniformFloatCount` / `Camera.uniformByteSize()`

Uniform packing constants for camera-related shaders:

- `uniformFloatCount`: `24`
- `uniformByteSize()`: `96` bytes

Layout:

- `mat4 viewProj` (16 floats)
- `vec4 cameraRight` (4 floats, `w = 0`)
- `vec4 cameraUp` (4 floats, `w = 0`)

### `writeUniformData(out, offset?)`

Writes the packed camera uniform layout above into `out` (with optional float offset).

This is compatible with both `PerspectiveCamera` and `OrthographicCamera` because both use the same orthogonal view basis from `lookAt`.

### `getPosition()`

Copy of the last `lookAt` eye position.

### `getLookAtTarget()`

Copy of the last `lookAt` target.

### `getFieldOfView()`

Returns `undefined` on the base class. `PerspectiveCamera` returns FOV in **radians**.

## Subclasses

- [PerspectiveCamera](PerspectiveCamera.md) — perspective projection
- [OrthographicCamera](OrthographicCamera.md) — orthographic projection

## See also

- [Example: camera-triangle](../../examples/camera-triangle/src/main.ts)
