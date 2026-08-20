# Implementation: Texture

**Slug:** `texture`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-24

## Summary

Adds `Texture` for loading images into WebGPU (`GPUTexture` + view + sampler), scene bind-group layouts for textured draws (`viewProj` + texture + sampler), and `createPlaneTriangleList` for centered quads. New **texture** example: orbital camera, RGB axes, portrait image on an XY plane at the origin.

## What changed

### New modules

| File                                                                                    | Purpose                              |
| --------------------------------------------------------------------------------------- | ------------------------------------ |
| [`packages/belfast/src/core/Texture.ts`](../../../packages/belfast/src/core/Texture.ts) | `load`, `fromBitmap`, `destroy`      |
| [`packages/belfast/src/geom/plane.ts`](../../../packages/belfast/src/geom/plane.ts)     | Centered plane → triangle list + UVs |

### Modified modules

| File                                                                                                | Change                                                                  |
| --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| [`packages/belfast/src/helper/sceneLayout.ts`](../../../packages/belfast/src/helper/sceneLayout.ts) | `createSceneTextureBindGroupLayout`, `createSceneTexturePipelineLayout` |
| [`packages/belfast/src/index.ts`](../../../packages/belfast/src/index.ts)                           | Export `Texture`, plane geom, texture layout helpers                    |
| [`examples/texture/`](../../../examples/texture/)                                                   | Texture demo (`public/image.jpg`, orbital + axes)                       |

### Documentation

| File                                          | Purpose          |
| --------------------------------------------- | ---------------- |
| [`docs/api/Texture.md`](../../api/Texture.md) | API reference    |
| [`docs/api/README.md`](../../api/README.md)   | Export table     |
| [`docs/overview.md`](../../overview.md)       | Textures section |

## API design decisions

### 1. Single bind group (group 0)

Bindings 0–2: `viewProj` uniform, `texture_2d`, `sampler`. Binding 0 matches `createSceneUniformBindGroupLayout` so `AxisHelper` and textured draws share one bind group per frame.

### 2. `Texture.load` → `createImageBitmap`

Standard browser decode path; `copyExternalImageToTexture` upload (queued, no `onSubmittedWorkDone` CPU stall). Destination textures use `COPY_DST | TEXTURE_BINDING | RENDER_ATTACHMENT` (required by browsers for the GPU conversion fast path). Default `flipY: true`. No mipmaps in v1.

## Post-review fixes (Antigravity)

- Removed `await queue.onSubmittedWorkDone()` from `fromBitmap` (non-blocking upload)

### 3. XY plane at origin

Portrait image on vertical `xy` plane (`z = 0`), aspect preserved via `planeW` / `planeH`. `cullMode: "none"` so visible when orbiting behind the plane.

### 4. Plane geometry internal export

`createPlaneTriangleList` exported for examples; expanded triangles (no index buffer).

## Example: texture

| Item     | Detail                                             |
| -------- | -------------------------------------------------- |
| Asset    | `examples/texture/public/image.jpg` → `/image.jpg` |
| Run      | `pnpm dev:example texture`                         |
| Controls | `OrbitalControl` (drag orbit, wheel zoom)          |

## Verification

```bash
pnpm --filter belfast typecheck
pnpm --filter belfast build
pnpm --filter @belfast/example-texture typecheck
pnpm dev:example texture
```

## Out of scope

- Mipmaps, cubemaps, render-target textures
- `TexturePlaneHelper` / fullscreen blit
- `drawIndexed`
