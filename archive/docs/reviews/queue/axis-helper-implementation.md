# Implementation: AxisHelper

**Slug:** `axis-helper`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-24

## Summary

Adds `AxisHelper` — three RGB axis lines (X red, Y green, Z blue) using `line-list` topology, based on alfrid [`DrawAxis`](../../../packages/alfrid/src/helper/DrawAxis.js). Updates **camera-orbit** to draw axes behind the triangle with a shared view-projection bind group.

## What changed

### New modules

| File                                                                                              | Purpose                                       |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| [`packages/belfast/src/helper/AxisHelper.ts`](../../../packages/belfast/src/helper/AxisHelper.ts) | Mesh + WGSL + `line-list` `Draw` for RGB axes |

### Modified modules

| File                                                                              | Change                                         |
| --------------------------------------------------------------------------------- | ---------------------------------------------- |
| [`packages/belfast/src/index.ts`](../../../packages/belfast/src/index.ts)         | Export `AxisHelper`, `AxisHelperOptions`       |
| [`examples/camera-orbit/src/main.ts`](../../../examples/camera-orbit/src/main.ts) | Instantiate `AxisHelper`, draw before triangle |

### Documentation

| File                                                | Purpose         |
| --------------------------------------------------- | --------------- |
| [`docs/api/AxisHelper.md`](../../api/AxisHelper.md) | API reference   |
| [`docs/api/README.md`](../../api/README.md)         | Export table    |
| [`docs/overview.md`](../../overview.md)             | Helpers section |

## API design decisions

### 1. Large default length (alfrid parity)

Default `length` is **1000** (±1000 per axis), same as alfrid `DrawAxis`. Lines extend far from the origin so axes read as “infinite” guides; only the portion inside the camera near/far range is visible.

### 2. Embedded WGSL + shared pipeline layout

Shader source lives in `AxisHelper.ts` as a string constant (no `?raw` in the library build). Uses the same `SceneUniforms { viewProj }` block as the camera-orbit triangle shader.

WebGPU `layout: "auto"` creates a **per-pipeline** bind group layout, so one bind group cannot be shared across `AxisHelper` and a triangle `Draw`. Belfast exports `createSceneUniformPipelineLayout` / `createSceneUniformBindGroupLayout`; pass `pipelineLayout` into both draws and create the bind group from `bindGroupLayout`.

### 3. `line-list` vs `triangle-list`

Axes need a separate `Draw` / pipeline from mesh draws because primitive topology differs. `AxisHelper.draw(pass, bindGroup)` wraps the internal pipeline; `getBindGroupLayout()` is exposed for layout checks.

### 4. No opacity uniform (v1)

alfrid `DrawAxis` uses `uOpacity` (0.75). Omitted here; fragment output is `vec4(color, 1.0)`.

## Geometry

| Axis | Endpoints              | Color |
| ---- | ---------------------- | ----- |
| X    | `(-L,0,0)` — `(L,0,0)` | Red   |
| Y    | `(0,-L,0)` — `(0,L,0)` | Green |
| Z    | `(0,0,-L)` — `(0,0,L)` | Blue  |

6 vertices, no index buffer.

## Example: camera-orbit

| Item     | Detail                                                       |
| -------- | ------------------------------------------------------------ |
| Run      | `pnpm dev:example camera-orbit`                              |
| Behavior | Drag orbit, wheel zoom, Shift/middle pan; RGB axes at origin |

```ts
const axes = new AxisHelper(device);
// same uniformBuffer + bindGroup as triangle
axes.draw(pass, bindGroup);
draw.draw(pass, mesh, bindGroup);
```

## Post-review fixes (Antigravity)

- `AxisHelper.destroy()` releases position/color GPU buffers
- `camera-orbit` calls `axes.destroy()` on `beforeunload`

## Verification

```bash
pnpm --filter belfast typecheck
pnpm --filter belfast build
pnpm --filter @belfast/example-camera-orbit typecheck
pnpm dev:example camera-orbit
```

Manual: red/green/blue lines from origin along X/Y/Z; triangle still visible at center.
