# Implementation: UniformBlock v1

**Slug:** `uniformblock`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-28

## Summary

Adds a reusable `UniformBlock` API for named uniform-buffer packing with explicit schema.  
v1 targets flat field types (`f32`, `vec2f`, `vec3f`, `vec4f`, `mat4x4f`) and removes manual float-index writes in the depth-to-texture example.

## What changed

| File                                                                                              | Change                                                                                              |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [`packages/belfast/src/core/UniformBlock.ts`](../../../packages/belfast/src/core/UniformBlock.ts) | New core utility with schema-driven layout, alignment, `set`, `toFloat32Array`, and `writeToBuffer` |
| [`packages/belfast/src/index.ts`](../../../packages/belfast/src/index.ts)                         | Exported `UniformBlock`, `UniformFieldType`, `UniformBlockSchema`                                   |
| [`examples/depth-to-texture/src/main.ts`](../../../examples/depth-to-texture/src/main.ts)         | Replaced manual scene uniform float indexing with named `UniformBlock` updates                      |
| [`docs/api/UniformBlock.md`](../../../docs/api/UniformBlock.md)                                   | New API docs page                                                                                   |
| [`docs/api/README.md`](../../../docs/api/README.md)                                               | Added UniformBlock exports                                                                          |
| [`docs/overview.md`](../../../docs/overview.md)                                                   | Added UniformBlock usage note in uniforms section                                                   |

## API design

- Explicit schema first (safety and predictable layout):
  - `UniformBlock.create({ viewProj: "mat4x4f", lightDir: "vec4f" })`
- Named updates:
  - `block.set("viewProj", matrix)`
  - `block.set("lightDir", [-0.6, -0.7, -0.4, 0])`
- One packed backing array:
  - `block.toFloat32Array()` for direct buffer writes
  - `block.writeToBuffer(buffer, device)` convenience path

## Alignment behavior (v1)

- Applies WGSL uniform alignment for supported flat fields.
- `vec3f` stores in a 16-byte slot (padded to 4 floats).
- `mat4x4f` occupies 64 bytes (16 floats).

## Why this change

- Eliminates magic indices like `sceneUniformData[32] = ...`
- Makes example code self-documenting via uniform names
- Preserves performance characteristics (single packed typed array write)

## Validation plan

```bash
pnpm --filter belfast typecheck
pnpm --filter belfast build
pnpm --filter @belfast/example-depth-to-texture typecheck
pnpm --filter @belfast/example-depth-to-texture build
```

Manual smoke:

- `pnpm dev:example depth-to-texture`
- Confirm lit cube renders and depth preview still updates
