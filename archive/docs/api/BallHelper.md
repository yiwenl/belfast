# BallHelper

Unit-sphere debug mesh with per-draw position, scale, color, and opacity (alfrid `DrawBall` parity).

## Import

```ts
import {
  BallHelper,
  type BallHelperOptions,
  type BallDrawParams,
  createSceneBallPipelineLayout,
} from "belfast";
```

## Constructor

```ts
new BallHelper(device: Device, options?: BallHelperOptions)
```

| Option           | Default        | Description                                           |
| ---------------- | -------------- | ----------------------------------------------------- |
| `radius`         | `1`            | Base mesh radius (`Geom.sphere(1, …)`)                |
| `segments`       | `12`           | Lat/long subdivisions                                 |
| `label`          | `"BallHelper"` | GPU debug labels                                      |
| `pipelineLayout` | auto           | From `createSceneBallPipelineLayout().pipelineLayout` |

Uses **two bind groups**: group 0 = shared scene `viewProj`; group 1 = instance uniforms updated each `draw()`.

The pipeline uses alpha blending and **does not write depth** (`depthWriteEnabled: false`). Draw opaque geometry first, then call `ball.draw()` so transparency composites correctly.

## `draw(pass, sceneBindGroup, params?)`

**One sphere per `BallHelper` per frame.** Each call writes the same instance uniform buffer; multiple `draw()` calls in one frame all use the last written values. Use multiple `BallHelper` instances for multiple spheres (instanced `drawMany` may come later).

| Param      | Default   | Maps to                              |
| ---------- | --------- | ------------------------------------ |
| `position` | `[0,0,0]` | `translate`                          |
| `scale`    | `1`       | `vec3` or uniform number             |
| `color`    | `[1,1,1]` | RGB                                  |
| `opacity`  | `1`       | Alpha (pipeline uses alpha blending) |

```ts
ball.draw(pass, sceneBindGroup, {
  position: [0, 0, 0],
  scale: 0.15,
  color: [1, 1, 1],
  opacity: 0.6,
});
```

## Shared camera bind group

Group 0 layout matches `createSceneUniformBindGroupLayout` — reuse the same `uniformBuffer` / `BindGroup` as triangle and `AxisHelper` draws.

`BallHelper` uses its own two-group **pipeline** layout; triangle/axes stay on the single-group scene layout.

## Methods

| Method                                | Description                               |
| ------------------------------------- | ----------------------------------------- |
| `draw(pass, sceneBindGroup, params?)` | Write instance uniforms and draw sphere   |
| `destroy()`                           | Release position and instance GPU buffers |

See [camera-orbit example](../../examples/camera-orbit/src/main.ts).
