# Implementation: Instancing planes example

**Slug:** `instancing-planes`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-28

## Summary

Adds a new example, **`instancing-planes`**, that renders a very large number of small planes with **one instanced draw call**.  
Each instance has its own random position, size, and color. The scene includes `OrbitalControl` and `AxisHelper`.

## What changed

### New example files

| File                                                                                                                                    | Purpose                                   |
| --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| [`examples/instancing-planes/package.json`](../../../examples/instancing-planes/package.json)                                           | Example package metadata                  |
| [`examples/instancing-planes/index.html`](../../../examples/instancing-planes/index.html)                                               | Canvas host page                          |
| [`examples/instancing-planes/tsconfig.json`](../../../examples/instancing-planes/tsconfig.json)                                         | TS config                                 |
| [`examples/instancing-planes/vite.config.ts`](../../../examples/instancing-planes/vite.config.ts)                                       | Vite setup via shared config              |
| [`examples/instancing-planes/src/vite-env.d.ts`](../../../examples/instancing-planes/src/vite-env.d.ts)                                 | Vite types                                |
| [`examples/instancing-planes/src/main.ts`](../../../examples/instancing-planes/src/main.ts)                                             | Instancing setup, camera, render loop     |
| [`examples/instancing-planes/src/shaders/instanced-planes.wgsl`](../../../examples/instancing-planes/src/shaders/instanced-planes.wgsl) | Vertex/fragment WGSL for instanced planes |

### Modified library files

| File                                                                                      | Change                                                                                             |
| ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| [`packages/belfast/src/camera/Camera.ts`](../../../packages/belfast/src/camera/Camera.ts) | Added reusable camera uniform packing (`uniformFloatCount`, `uniformByteSize`, `writeUniformData`) |
| [`pnpm-lock.yaml`](../../../pnpm-lock.yaml)                                               | Added importer entry for new example                                                               |

## Data layout

Instance buffer uses `stepMode: "instance"` and packs:

- `@location(1)`: `vec4<f32>` = `position.xyz + size`
- `@location(2)`: `vec4<f32>` = `color.rgb + alpha`

Static vertex buffer (`stepMode: "vertex"`) provides local plane vertices at `@location(0)`.

## Rendering approach

- Single `Mesh` with:
  - slot 0: plane vertex positions
  - slot 1: instance attributes
- One pipeline + one bind group
- One call: `draw.draw(pass, mesh, bindGroup, INSTANCE_COUNT)`
- `AxisHelper` is drawn for orientation
- `OrbitalControl` drives camera interaction

## Camera-facing planes (billboarding)

To keep each plane facing the camera:

- Shader uses camera basis vectors (`cameraRight`, `cameraUp`) from uniforms.
- Vertex world position is computed as:
  - `center + right * localX * size + up * localY * size`
- Camera uniform packing logic was moved into `Camera.writeUniformData(...)` to avoid repeated per-example matrix indexing.

## Performance notes

- Instance attributes are generated once and uploaded once (no per-frame instance buffer writes).
- Per-frame updates are limited to camera uniform data.
- `cullMode` is set to `"back"` for reduced raster cost.

## Verification

```bash
pnpm --filter belfast typecheck
pnpm --filter @belfast/example-instancing-planes typecheck
pnpm --filter belfast build
pnpm dev:example instancing-planes
```

Manual checks:

- Large instance count renders in a single instanced draw
- Planes have varied size/color/position
- Planes face the camera while orbiting
- Axis helper visible and stable

## Out of scope

- GPU-driven culling or LOD
- Instanced texture sampling for each plane
- Compute-based simulation or animation of instance data
