# Ray

A ray defined by an origin and a direction vector. Used for picking and hit-testing against triangles and spheres.

## Import

```ts
import { Ray } from "belfast";
```

## Constructor

```ts
new Ray(origin: ReadonlyVec3, direction: ReadonlyVec3)
```

Both `origin` and `direction` are cloned internally.

## Methods

### `set(origin, direction)`

Mutate origin and direction in-place. Returns `this`.

### `at(t, out?)`

Returns the point along the ray at parameter `t`: `origin + direction * t`. If `out` is provided, writes into it; otherwise uses an internal scratch vector.

### `intersectTriangle(pa, pb, pc, backfaceCulling?)`

Möller–Trumbore ray–triangle intersection. Returns a **newly allocated** `vec3` hit point, or `null` if no intersection.

`backfaceCulling` defaults to `true` — set to `false` to hit both sides.

```ts
const ray = new Ray([0, 5, 0], [0, -1, 0]);
const hit = ray.intersectTriangle([-1, 0, -1], [1, 0, -1], [0, 0, 1]);
// hit ≈ [0, 0, -0.333]
```

### `intersectSphere(center, radius)`

Returns the closest intersection point (`vec3`) or `null`.

```ts
const hit = ray.intersectSphere([0, 0, 0], 1.0);
```

## Properties

| Property    | Type   | Description                       |
| ----------- | ------ | --------------------------------- |
| `origin`    | `vec3` | Ray origin (mutable)              |
| `direction` | `vec3` | Ray direction (mutable, unit-len) |

## See also

- [Camera.generateRay](Camera.md) — unproject mouse to world-space ray
- [HitTestor](HitTestor.md) — event-driven hit testing
