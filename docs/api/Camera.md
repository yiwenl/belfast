# Camera

Base camera with view and projection matrices (CPU-side math, no GPU resources).

## Import

```ts
import { Camera } from "belfast";
import type { Vec3, Mat4 } from "belfast";
```

## Methods

### `lookAt(eye, target, up?)`

Builds the view matrix from eye position, look-at target, and up vector (default `[0, 1, 0]`). Stores values used by `getPosition()` and `getLookAtTarget()`.

### `getViewMatrix()`

Column-major `Float32Array` (16 elements), WGSL-compatible.

### `getProjectionMatrix()`

Projection matrix from the subclass (`PerspectiveCamera` or `OrthographicCamera`).

### `getViewProjectionMatrix(out?)`

Returns `projection * view` for `clip = viewProj * vec4(position, 1.0)`. Reuses an internal buffer when `out` is omitted.

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
