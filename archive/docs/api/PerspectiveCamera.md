# PerspectiveCamera

Extends [Camera](Camera.md) with a perspective projection matrix.

## Import

```ts
import { PerspectiveCamera } from "belfast";
```

## Constructor

```ts
new PerspectiveCamera(fov, aspect, near, far);
```

| Argument | Description                           |
| -------- | ------------------------------------- |
| `fov`    | Vertical field of view in **radians** |
| `aspect` | Width / height                        |
| `near`   | Near clip distance                    |
| `far`    | Far clip distance                     |

## Methods

### `setPerspective(fov, aspect, near, far)`

Rebuilds the projection matrix.

### `setAspect(aspect)`

Updates aspect ratio (call after canvas resize).

### `getFieldOfView()`

Returns vertical FOV in radians.

### `getAspect()`, `getNear()`, `getFar()`

Current projection parameters.

## Example

```ts
const camera = new PerspectiveCamera(Math.PI / 4, width / height, 0.1, 100);
camera.lookAt([0, 0, 2], [0, 0, 0]);
const viewProj = camera.getViewProjectionMatrix();
uniformBuffer.write(device, viewProj);
```
