# Implementation: BallHelper

**Slug:** `ball-helper`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-24

## Summary

Adds `BallHelper` — a unit-sphere mesh with per-draw `position`, `scale`, `color`, and `opacity` (alfrid [`DrawBall`](../../../packages/alfrid/src/helper/DrawBall.js) parity). Sphere geometry is expanded to a triangle list (no index buffer). Two bind groups: shared scene `viewProj` (group 0) + instance uniforms (group 1). **camera-orbit** draws a semi-transparent ball at the origin.

## What changed

### New modules

| File                                                                                              | Purpose                                                           |
| ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| [`packages/belfast/src/geom/sphere.ts`](../../../packages/belfast/src/geom/sphere.ts)             | `createSphereTriangleList` — alfrid `Geom.sphere` without indices |
| [`packages/belfast/src/helper/BallHelper.ts`](../../../packages/belfast/src/helper/BallHelper.ts) | Sphere mesh, WGSL, instance uniform, `draw` / `destroy`           |

### Modified modules

| File                                                                                                | Change                                                               |
| --------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| [`packages/belfast/src/helper/sceneLayout.ts`](../../../packages/belfast/src/helper/sceneLayout.ts) | `createBallInstanceBindGroupLayout`, `createSceneBallPipelineLayout` |
| [`packages/belfast/src/helper/Draw.ts`](../../../packages/belfast/src/helper/Draw.ts)               | `draw()` accepts `BindGroup \| BindGroup[]`                          |
| [`packages/belfast/src/index.ts`](../../../packages/belfast/src/index.ts)                           | Export `BallHelper`, layout helpers                                  |
| [`examples/camera-orbit/src/main.ts`](../../../examples/camera-orbit/src/main.ts)                   | Ball at origin, scale 0.15, opacity 0.6                              |

### Documentation

| File                                                | Purpose       |
| --------------------------------------------------- | ------------- |
| [`docs/api/BallHelper.md`](../../api/BallHelper.md) | API reference |
| [`docs/api/README.md`](../../api/README.md)         | Export table  |
| [`docs/overview.md`](../../overview.md)             | Debug helpers |

## API design decisions

### 1. No `drawIndexed` (v1)

alfrid uses indexed sphere quads. Belfast expands each quad to two triangles × 3 vertices in CPU memory (~2592 vertices for 12 segments). Keeps public API free of index buffers for now.

### 2. Two bind groups

- **Group 0:** `viewProj` — same layout as `createSceneUniformBindGroupLayout`; reuse triangle/axes `BindGroup`.
- **Group 1:** `BallInstance` uniform (translate, scale, color, opacity) — written each `draw()`.

`createSceneBallPipelineLayout` builds a 2-group `GPUPipelineLayout`. Triangle and `AxisHelper` remain on the 1-group scene layout.

### 3. Per-draw params (alfrid `draw(pos, scale, color, opacity)`)

```ts
ball.draw(pass, sceneBindGroup, {
  position: [0, 0, 0],
  scale: 0.15, // number or Vec3
  color: [1, 1, 1],
  opacity: 0.6,
});
```

Rotation (`uRotation` in alfrid `general.vert`) omitted — not set by `DrawBall.draw()`.

### 4. Alpha blending and depth

Ball pipeline enables standard alpha blend on the swapchain format so `opacity < 1` composites correctly.

**`depthWriteEnabled: false`** — transparent draws must not write the depth buffer (otherwise opaque geometry drawn afterward shows striped z-fighting / incorrect occlusion). **Draw order:** opaque (triangle, axes) first, then `ball.draw()` last.

### 5. `destroy()`

Releases position buffer and instance uniform buffer (same lifecycle pattern as `AxisHelper`).

## Example: camera-orbit

| Item       | Detail                                             |
| ---------- | -------------------------------------------------- |
| Run        | `pnpm dev:example camera-orbit`                    |
| Draw order | axes → triangle → ball (opaque before transparent) |

## Verification

```bash
pnpm --filter belfast typecheck
pnpm --filter belfast build
pnpm --filter @belfast/example-camera-orbit typecheck
pnpm dev:example camera-orbit
```

Manual: semi-transparent white ball at origin; RGB axes; colored triangle; orbit controls work.

## Post-review fixes (Antigravity)

- Document single-draw-per-instance uniform overwrite hazard
- `geom/sphere.ts`: symmetric `Math.round` instead of `Math.floor`

## Out of scope

- `Geom` module export / `drawIndexed`
- Instanced `drawMany` (multiple spheres per helper)
- Per-draw rotation
- Lighting / textures
