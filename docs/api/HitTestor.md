# HitTestor

CPU-side hit tester that casts a ray from the camera through the mouse cursor and tests it against the triangles of a `GeometryData` object. Extends the native browser `EventTarget`.

## Import

```ts
import { HitTestor } from "belfast";
import type { HitTestorOptions, HitDetail } from "belfast";
```

## Constructor

```ts
new HitTestor(
  geometry: GeometryData,
  camera: Camera,
  resolution?: [number, number],
  options?: HitTestorOptions,
)
```

| Parameter    | Default                            | Description                                |
| ------------ | ---------------------------------- | ------------------------------------------ |
| `geometry`   | —                                  | CPU-side geometry from `Geom.plane()` etc. |
| `camera`     | —                                  | Camera used for ray unprojection           |
| `resolution` | `[window.innerWidth, innerHeight]` | Viewport size in pixels                    |
| `options`    | `{}`                               | See `HitTestorOptions` below               |

The constructor calls `connect()` automatically.

### `HitTestorOptions`

```ts
interface HitTestorOptions {
  skipMoveCheck?: boolean; // skip hit-testing on mousemove (default false)
  listenerTarget?: EventTarget; // element to listen on (default window)
}
```

## Events

Events are dispatched as `CustomEvent<HitDetail>` via the standard `EventTarget` API.

| Event name | Fires when                                        | `detail`        |
| ---------- | ------------------------------------------------- | --------------- |
| `"onHit"`  | Mouse moves and hits the geometry                 | `{ hit: vec3 }` |
| `"onDown"` | Mouse is pressed down and hits the geometry       | `{ hit: vec3 }` |
| `"onUp"`   | Mouse released (click within tolerance) or no hit | —               |

```ts
hitTestor.addEventListener("onHit", (e) => {
  const { hit } = (e as CustomEvent<HitDetail>).detail;
  console.log("hover at", hit);
});
```

### `HitDetail`

```ts
interface HitDetail {
  hit: vec3; // world-space intersection point
}
```

## Methods

### `connect()`

Attach mouse and touch listeners. Called automatically in the constructor.

### `disconnect()`

Remove all event listeners from the target element.

## Properties

| Property         | Type               | Description                                            |
| ---------------- | ------------------ | ------------------------------------------------------ |
| `modelMatrix`    | `mat4`             | World transform applied to geometry before testing     |
| `resolution`     | `[number, number]` | Viewport size in pixels                                |
| `clickTolerance` | `number`           | Pixel distance threshold for click vs drag (default 8) |
| `hit`            | `Readonly<vec3>`   | Last successful hit position (read-only getter)        |

## Usage

```ts
import { Geom, PerspectiveCamera, HitTestor } from "belfast";
import { mat4 } from "gl-matrix";

const geom = Geom.plane({ width: 4, height: 4 });
const camera = new PerspectiveCamera(Math.PI / 4, canvas.width / canvas.height, 0.1, 100);
camera.lookAt([0, 5, 10], [0, 0, 0]);

const hitTestor = new HitTestor(geom, camera, [canvas.width, canvas.height]);

// Apply a model transform (optional)
mat4.translate(hitTestor.modelMatrix, hitTestor.modelMatrix, [0, 1, 0]);

hitTestor.addEventListener("onDown", (e) => {
  const { hit } = (e as CustomEvent).detail;
  console.log("clicked at", hit);
});

hitTestor.addEventListener("onHit", (e) => {
  const { hit } = (e as CustomEvent).detail;
  // highlight hovered face, show cursor, etc.
});

hitTestor.addEventListener("onUp", () => {
  // clear highlight
});

// Cleanup
hitTestor.disconnect();
```

## Design notes

- Accepts `GeometryData` (from `Geom.plane()`, `Geom.sphere()`, `Geom.cube()`) rather than a GPU `Mesh`, since hit testing is CPU-only and avoids GPU readback.
- Face triples are pre-built from positions + indices at construction time.
- Uses `Camera.generateRay()` to unproject the mouse position into a world-space ray.
- Touch events are supported alongside mouse events.

## See also

- [Ray](Ray.md) — the ray math used internally
- [Camera.generateRay](Camera.md) — ray unprojection
- [Geom](Geom.md) — geometry data generation
