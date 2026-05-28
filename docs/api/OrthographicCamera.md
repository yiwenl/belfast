# OrthographicCamera

Extends [Camera](Camera.md) with an orthographic projection matrix (WebGPU clip-space Z in `[0, 1]`).

## Import

```ts
import { OrthographicCamera } from "belfast";
```

## Constructor

```ts
new OrthographicCamera(left, right, bottom, top, near?, far?)
```

| Argument | Default | Description          |
| -------- | ------- | -------------------- |
| `left`   | —       | Left frustum plane   |
| `right`  | —       | Right frustum plane  |
| `bottom` | —       | Bottom frustum plane |
| `top`    | —       | Top frustum plane    |
| `near`   | `0.1`   | Near clip            |
| `far`    | `100`   | Far clip             |

Parameter order matches `gl-matrix` `mat4.orthoZO(left, right, bottom, top, near, far)`.

## Methods

### `setOrthographic(left, right, bottom, top, near?, far?)`

Rebuilds the projection matrix.

### `getFieldOfView()`

Returns `undefined` (no perspective FOV).

## Example

```ts
const camera = new OrthographicCamera(-1, 1, -1, 1, 0.1, 100);
camera.lookAt([0, 0, 5], [0, 0, 0]);
```
