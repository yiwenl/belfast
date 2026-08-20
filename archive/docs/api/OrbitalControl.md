# OrbitalControl

Orbits a [Camera](Camera.md) around a center point with mouse/touch drag and wheel zoom. Inspired by alfrid `OrbitalControl`.

## Import

```ts
import { OrbitalControl, type OrbitalControlOptions } from "belfast";
```

Uses [`scheduling`](https://www.npmjs.com/package/scheduling) to update camera pose each enterframe — call `camera.getViewProjectionMatrix()` in your render loop; no `control.update()` required.

## Constructor

```ts
new OrbitalControl(camera, options?)
```

### `OrbitalControlOptions`

| Field            | Default         | Description                                                |
| ---------------- | --------------- | ---------------------------------------------------------- |
| `listenerTarget` | `document.body` | Element receiving pointer/wheel events                     |
| `center`         | `[0, 0, 0]`     | Orbit pivot / look-at target                               |
| `radius`         | `10`            | Initial distance from center                               |
| `up`             | `[0, 1, 0]`     | Camera up vector                                           |
| `sensitivity`    | `1`             | Drag sensitivity multiplier                                |
| `zoomSpeed`      | `1`             | Wheel zoom multiplier (trackpad-friendly with `deltaMode`) |
| `panSpeed`       | `0.01`          | Middle-mouse / Shift+drag pan scale                        |

Automatically calls `connect()` and registers an enterframe loop.

## Methods

| Method                                                   | Description                                                                       |
| -------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `connect()` / `disconnect()`                             | Add/remove DOM listeners                                                          |
| `destroy()`                                              | Disconnect + remove Scheduler callbacks + destroy internal `EaseNumber` instances |
| `update()`                                               | Recompute position only (camera updates on next enterframe)                       |
| `lock()` / `lockZoom()` / `lockRotation()` / `lockPan()` | Disable interaction                                                               |
| `inverseControl(invert?)`                                | Flip drag direction                                                               |

## Properties

| Property         | Description                            |
| ---------------- | -------------------------------------- |
| `center`         | Orbit pivot (mutable)                  |
| `radius`         | `EaseNumber` for zoom distance         |
| `position`       | Current eye position after last update |
| `positionOffset` | Added to computed position             |
| `sensitivity`    | Drag scale                             |
| `rx` / `ry`      | `EaseNumber` for pitch and yaw         |

## Interaction

- **Drag** — orbit (pitch clamped to ±90°)
- **Wheel** — zoom in/out (`deltaMode`-aware; tune with `zoomSpeed`)
- **Middle-mouse or Shift+drag** — pan orbit pivot (`center`)
- **Touch** — orbit drag; `preventDefault` while dragging

## Example

```ts
const camera = new PerspectiveCamera(Math.PI / 4, aspect, 0.1, 100);
const control = new OrbitalControl(camera, {
  listenerTarget: canvas,
  center: [0, 0, 0],
  radius: 2.5,
});

window.addEventListener("beforeunload", () => control.destroy());

// render loop
const viewProj = camera.getViewProjectionMatrix();
```

See [camera-orbit example](../../examples/camera-orbit/src/main.ts).

## See also

- [EaseNumber](EaseNumber.md)
- [PerspectiveCamera](PerspectiveCamera.md)
