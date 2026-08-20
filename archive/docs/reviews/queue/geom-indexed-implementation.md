# Implementation: Geom with indexed rendering

**Slug:** `geom-indexed`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-28

## Summary

Adds indexed rendering support to Belfast core and introduces public `Geom` primitive generation APIs:

- Core indexed draw path (`BufferUsage.index`, `Mesh.setIndexBuffer`, automatic `drawIndexed`)
- New `Geom` helper with `plane`, `sphere`, and `cube` generators returning `positions`/`uvs`/`normals`/`indices`
- New `geom-indexed` example rendering an indexed lit cube with orbit camera
- API docs/overview updates for indexed workflows and new exports

## What changed

### Core API

| File                                                                                  | Change                                                                                                          |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| [`packages/belfast/src/core/Buffer.ts`](../../../packages/belfast/src/core/Buffer.ts) | Added `BufferUsage.index` preset (`INDEX \| COPY_DST`)                                                          |
| [`packages/belfast/src/core/Mesh.ts`](../../../packages/belfast/src/core/Mesh.ts)     | Added optional index buffer state via `setIndexBuffer(buffer, count, format)` and index binding in `bind(pass)` |
| [`packages/belfast/src/helper/Draw.ts`](../../../packages/belfast/src/helper/Draw.ts) | `draw(...)` now calls `drawIndexed(...)` automatically when mesh has indices                                    |

### Geom utility

| File                                                                                  | Change                                                                               |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| [`packages/belfast/src/helper/Geom.ts`](../../../packages/belfast/src/helper/Geom.ts) | New `Geom` class with `plane`, `sphere`, `cube` indexed generators plus option/types |
| [`packages/belfast/src/index.ts`](../../../packages/belfast/src/index.ts)             | Exported `Geom`, geometry types/options, and `MeshIndexFormat`                       |

### Example

| File                                                                                                                  | Purpose                                                                           |
| --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| [`examples/geom-indexed/src/main.ts`](../../../examples/geom-indexed/src/main.ts)                                     | Indexed cube render flow using `Geom.cube`, `Mesh.setIndexBuffer`, and lit shader |
| [`examples/geom-indexed/src/shaders/geom-indexed.wgsl`](../../../examples/geom-indexed/src/shaders/geom-indexed.wgsl) | Diffuse lighting shader with position/normal attributes                           |
| [`examples/geom-indexed/`](../../../examples/geom-indexed/)                                                           | New runnable example scaffold                                                     |

### Docs

| File                                                | Change                                                               |
| --------------------------------------------------- | -------------------------------------------------------------------- |
| [`docs/api/Geom.md`](../../../docs/api/Geom.md)     | New Geom API page                                                    |
| [`docs/api/Mesh.md`](../../../docs/api/Mesh.md)     | Documented index buffer setup and indexed draw behavior              |
| [`docs/api/Draw.md`](../../../docs/api/Draw.md)     | Documented automatic `drawIndexed` path                              |
| [`docs/api/Buffer.md`](../../../docs/api/Buffer.md) | Added `BufferUsage.index` preset documentation                       |
| [`docs/api/README.md`](../../../docs/api/README.md) | Added Geom and mesh index exports                                    |
| [`docs/overview.md`](../../../docs/overview.md)     | Removed “index buffers not in public API” note; updated draw mapping |

## API decisions

1. **Indexed support is additive**
   - Existing non-indexed mesh usage remains unchanged.
   - `Draw.draw` keeps the same signature and chooses indexed vs non-indexed internally.

2. **`Mesh` owns index metadata**
   - `Mesh.setIndexBuffer` stores index buffer, count, and format.
   - Draw call sites do not need to branch manually on draw type.

3. **`Geom` returns typed arrays only**
   - Geometry generation stays CPU-side and renderer-agnostic.
   - Upload strategy (`Buffer.fromData`, usage flags, interleaved/non-interleaved layout) remains app-controlled.

4. **Automatic index type selection**
   - `Geom` emits `Uint16Array` when vertex count is <= 65535 and upgrades to `Uint32Array` otherwise.

## Verification plan

```bash
pnpm --filter belfast typecheck
pnpm --filter belfast build
pnpm --filter @belfast/example-geom-indexed typecheck
pnpm dev:example geom-indexed
```

Manual checks:

- Indexed cube renders correctly in `geom-indexed` example
- Orbit interaction remains smooth
- Existing non-indexed examples (`camera-orbit`, `texture`, `render-to-texture`) still work
- No API regressions for old `Draw.draw(pass, mesh)` and `Draw.draw(pass, vertexCount)` paths
