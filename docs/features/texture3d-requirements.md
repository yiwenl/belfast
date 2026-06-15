# Feature requirements: Texture3D

**Status:** Draft  
**Primary consumer:** `@fluid-sim-belfast` (3D stable-fluids sim in experiments2)  
**Related:** [Texture API](../api/Texture.md), [Compute API](../api/Compute.md)  
**Driver project:** `experiments2/libs/fluid-sim-belfast` + `experiments2/apps/fluid-sim-3d`

---

## 1. Background

Belfast currently exposes [`Texture`](../api/Texture.md) for **2D image upload** (`load`, `fromBitmap`) with `texture_2d` sampling. There is no wrapper for volumetric GPU data.

The first consumer is a **3D stable-fluids simulation** porting the 2D API in `experiments2/apps/window/src/fluid-sim/` from Alfrid/WebGL fragment passes to Belfast/WebGPU **compute shaders** on a 3D grid.

That simulation needs:

- Empty 3D volumes (no image source)
- **Compute write** (`texture_storage_3d`) and **shader sample** (`texture_3d`) on the same data
- **Ping-pong** between two volumes per simulation pass
- A **sampler** with boundary behavior suitable for PDE stencils (mirrored / clamped edges)
- Bind-group resources compatible with existing `Compute`, `Draw`, and `BindGroup` helpers

This document defines the minimum Belfast API to support that workload without leaking raw `GPUTexture` lifecycle into every consumer.

---

## 2. Goals

| #   | Goal                                                                                                      |
| --- | --------------------------------------------------------------------------------------------------------- |
| G1  | Provide a `Texture3D` class for creating and binding volumetric GPU textures                              |
| G2  | Support both **storage writes** (compute) and **filtered sampling** (render) on the same texture          |
| G3  | Provide a `Texture3DPingPong` helper matching the read/write/swap pattern used by Alfrid `FboPingPong`    |
| G4  | Mirror Belfast `Texture` conventions: `label`, `destroy()`, `view`, `sampler`, bind-group-ready resources |
| G5  | Keep v1 scope small — only what the fluid sim and its arrow visualization need                            |

---

## 3. Non-goals (v1)

- Loading 3D data from files (`.raw`, NIfTI, DICOM, etc.)
- 3D render targets / volume ray-marching helpers
- Mipmaps, cubemaps, texture arrays, compressed formats
- `copyExternalImageToTexture` or CPU upload helpers (beyond optional `writeData` — see §6.4)
- Automatic format/feature detection UI
- Integration with `RenderTarget` (2D offscreen)
- `Texture3DPlaneHelper` / slice blit utilities

---

## 4. Consumer use cases

### 4.1 Fluid simulation compute pipeline

**Grid:** `32×32×32` default (consumer may raise to `64³`; `128³` is aspirational, not a v1 requirement).

**Fields stored as separate 3D textures:**

| Field      | Channels used | Ping-pong?                                |
| ---------- | ------------- | ----------------------------------------- |
| Velocity   | `.xyz` = vec3 | Yes                                       |
| Density    | `.x` = scalar | Yes                                       |
| Pressure   | `.x` = scalar | Yes                                       |
| Divergence | `.x` = scalar | No (single buffer, overwritten each step) |

**Per-frame compute sequence** (each step = one dispatch + swap where applicable):

1. Advect velocity (read vel → write vel)
2. Advect density (read density → write density)
3. Divergence (read velocity → write divergence)
4. Clear pressure (read/write pressure with dissipation)
5. Jacobi × N iterations (read/write pressure, read divergence)
6. Gradient subtract (read pressure + velocity → write velocity)
7. Splat forces (read/write velocity and density, per force injection)

**Workgroup layout:** `4×4×4` threads, dispatch `(size / 4)³`.

**WGSL patterns required:**

```wgsl
// Compute pass — read prior field, write next field
@group(0) @binding(0) var velocityIn: texture_3d<f32>;
@group(0) @binding(1) var velocityOut: texture_storage_3d<write, rgba32float>;

// Render pass — sample field for visualization
@group(0) @binding(1) var velocityTex: texture_3d<f32>;
@group(0) @binding(2) var fieldSampler: sampler;
```

### 4.2 Arrow field visualization

A render pass draws an instanced `line-list` arrow grid (e.g. `16³` arrows). The vertex shader **samples** velocity and density 3D textures at grid-cell UVs in normalized `[0, 1]³` space:

- Arrow **direction** ← `normalize(velocity.xyz)`
- Arrow **length** ← `density.x * scale`

No CPU readback. The same `texture.view` + `texture.sampler` used in compute bind groups must work in a `Draw` bind group.

### 4.3 Ping-pong contract

Mirrors Alfrid [`FboPingPong`](../../packages/alfrid/src/helper/FboPingPong.js):

| Member   | Semantics                                                                  |
| -------- | -------------------------------------------------------------------------- |
| `read`   | Last completed write; bind as **input** (`texture_3d` / read-only storage) |
| `write`  | Target for current pass; bind as **output** (`texture_storage_3d` write)   |
| `swap()` | Rotate buffers after each pass so the written texture becomes `read`       |

---

## 5. Proposed public API

### 5.1 `Texture3D`

```ts
import { Texture3D, type Texture3DOptions } from "belfast";
```

#### `Texture3DOptions`

| Option         | Default           | Description                                         |
| -------------- | ----------------- | --------------------------------------------------- |
| `label`        | `"Texture3D"`     | GPU debug label                                     |
| `format`       | `"rgba32float"`   | `GPUTextureFormat`; must support storage + sample   |
| `usage`        | see §6.1          | Override only when needed                           |
| `addressModeU` | `"mirror-repeat"` | Sampler wrap U (fluid sim uses mirrored boundaries) |
| `addressModeV` | `"mirror-repeat"` | Sampler wrap V                                      |
| `addressModeW` | `"mirror-repeat"` | Sampler wrap W                                      |
| `magFilter`    | `"linear"`        | Magnification filter                                |
| `minFilter`    | `"linear"`        | Minification filter                                 |

> **Note:** 2D `Texture` defaults to `clamp-to-edge`. Fluid sim explicitly needs **mirrored** boundaries to match the 2D Alfrid sim (`GL.MIRRORED_REPEAT`). Default for `Texture3D` should be `mirror-repeat` unless overridden.

#### Static factory

```ts
Texture3D.create(
  device: Device,
  size: number | [number, number, number],
  options?: Texture3DOptions,
): Texture3D
```

- `size` as `number` → cubic volume `[n, n, n]`
- Creates GPU texture, default `view` (`GPUTextureViewDimension: "3d"`), and `sampler`
- Does **not** upload initial data (starts zero/uninitialized — acceptable for sim)

#### Instance members

| Member        | Type               | Description                                                              |
| ------------- | ------------------ | ------------------------------------------------------------------------ |
| `width`       | `number`           | Voxel width (X)                                                          |
| `height`      | `number`           | Voxel height (Y)                                                         |
| `depth`       | `number`           | Voxel depth (Z)                                                          |
| `format`      | `GPUTextureFormat` | Stored format                                                            |
| `view`        | `GPUTextureView`   | `texture_3d` / `texture_storage_3d` binding (default 3D view)            |
| `storageView` | `GPUTextureView`   | Explicit view for `texture_storage_3d<write, …>` if distinct from `view` |
| `sampler`     | `GPUSampler`       | For `textureSample` in render shaders                                    |
| `gpu`         | `GPUTexture`       | Escape hatch for advanced consumers                                      |

#### Methods

```ts
destroy(): void
```

Destroys the underlying `GPUTexture`. Sampler is not destroyed (matches `Texture`).

### 5.2 `Texture3DPingPong`

```ts
import { Texture3DPingPong, type Texture3DPingPongOptions } from "belfast";
```

```ts
Texture3DPingPong.create(
  device: Device,
  size: number | [number, number, number],
  options?: Texture3DPingPongOptions, // extends Texture3DOptions
): Texture3DPingPong
```

| Member      | Type        | Description                             |
| ----------- | ----------- | --------------------------------------- |
| `read`      | `Texture3D` | Input texture for current pass          |
| `write`     | `Texture3D` | Output texture for current pass         |
| `size`      | `number`    | Cube edge (when cubic) or max dimension |
| `swap()`    | `void`      | Exchange read/write after a pass        |
| `destroy()` | `void`      | Destroy both textures                   |

**Design note:** `read` and `write` return full `Texture3D` instances so consumers can pass `read.view` / `read.sampler` to render passes and `write.storageView` to compute passes without extra wrappers.

---

## 6. Technical requirements

### 6.1 GPU texture usage flags

Default usage for simulation volumes:

```
GPUTextureUsage.TEXTURE_BINDING |
GPUTextureUsage.STORAGE_BINDING
```

Do **not** require `RENDER_ATTACHMENT` or `COPY_DST` for v1 (unlike 2D `Texture`, which needs `RENDER_ATTACHMENT` for image upload).

### 6.2 Format

**Required v1 format:** `rgba32float`

- Velocity stored in `.xyz`, density/pressure/divergence in `.x`
- Must be creatable as both `texture_3d<f32>` and `texture_storage_3d<write, rgba32float>`

**Optional v1 format:** `r32float` for single-channel fields (pressure, density, divergence). Not required if `rgba32float` alone is sufficient.

### 6.3 Views

Each `Texture3D` must expose:

1. **`view`** — default `createView({ dimension: "3d" })` for `texture_3d` sampling
2. **`storageView`** — same underlying texture, valid for `texture_storage_3d<write, format>` in compute

If WebGPU allows a single view for both roles on the target platform, `storageView` may alias `view`. The API should hide that detail.

### 6.4 CPU upload (optional, low priority)

```ts
// Optional v1 — only if trivial to implement
writeData(device: Device, data: Float32Array, options?: { offset?: [number, number, number] }): void
```

Fluid sim v1 does **not** require CPU upload. Initialization is zero + splat forces. Defer unless needed for tests.

### 6.5 Sampler

3D sampler with configurable `addressModeU/V/W`. Fluid sim requires **`mirror-repeat`** on all three axes to match the 2D sim's mirrored boundary sampling in advection and stencil passes.

### 6.6 Bind group integration

Must work with existing helpers without new layout factories for v1:

```ts
// Compute bind group (fluid advect pass example)
BindGroup.create(device, compute.getBindGroupLayout(0), [
  { binding: 0, resource: velocityPingPong.read.view },
  { binding: 1, resource: velocityPingPong.write.storageView },
  { binding: 2, resource: uniformBuffer },
]);

// Draw bind group (arrow visualization)
BindGroup.create(device, draw.getBindGroupLayout(0), [
  { binding: 0, resource: uniformBuffer },
  { binding: 1, resource: fluid.velocity.view },
  { binding: 2, resource: fluid.density.view },
  { binding: 3, resource: fluid.velocity.sampler }, // shared sampler OK if formats match
]);
```

No requirement for `createSceneVolumePipelineLayout` in v1 — consumers define their own `Draw`/`Compute` bind layouts. A shared layout helper is **nice-to-have**, not required.

### 6.7 Lifecycle

- `Texture3D.create` allocates GPU resources synchronously (same as creating via `device.gpu.createTexture` today)
- `destroy()` on `Texture3D` and `Texture3DPingPong` must be idempotent-safe or documented as single-call
- No implicit global registry — consumers own lifecycle (matches `Texture`, `Buffer`)

### 6.8 Labels

All `GPUTexture`, `GPUTextureView`, and `GPUSampler` objects should receive debug labels derived from `options.label` (e.g. `"Velocity3D-read"`, `"Velocity3D-write"`).

---

## 7. Performance and scale requirements

| Parameter        | v1 target       | Notes                                              |
| ---------------- | --------------- | -------------------------------------------------- |
| Default grid     | `32³`           | ~32k voxels per field                              |
| Stretch goal     | `64³`           | ~262k voxels; should remain interactive on desktop |
| Aspirational     | `128³`          | Out of v1 acceptance criteria                      |
| Passes per frame | ~125 dispatches | 5 base passes + 20 Jacobi iterations + splats      |
| CPU readback     | None            | GPU-only sim + visualization                       |

Belfast `Texture3D` itself does not need to optimize the compute pipeline — but allocation and view creation must not add per-frame overhead (create once, bind many).

---

## 8. Relationship to existing `Texture`

| Aspect          | `Texture` (2D)        | `Texture3D` (proposed)     |
| --------------- | --------------------- | -------------------------- |
| Creation        | `load` / `fromBitmap` | `create` (empty volume)    |
| Primary use     | Image display         | Simulation fields          |
| Default wrap    | `clamp-to-edge`       | `mirror-repeat`            |
| Storage binding | No                    | Yes                        |
| Dimensions      | `width`, `height`     | `width`, `height`, `depth` |

Keep as **separate classes** — do not overload `Texture` with a `dimension` parameter. WGSL types, view creation, and upload paths differ enough to warrant distinct types.

---

## 9. Acceptance criteria

### 9.1 Unit / integration (Belfast)

- [ ] `Texture3D.create(device, 32)` allocates a `32×32×32` `rgba32float` texture with `TEXTURE_BINDING | STORAGE_BINDING`
- [ ] `view` binds in a `Compute` shader as `texture_3d<f32>`
- [ ] `storageView` binds in the same pipeline layout as `texture_storage_3d<write, rgba32float>` and a compute dispatch writes a known pattern
- [ ] A second `Draw` pass can `textureSample` the written data with `sampler` and produce visible output
- [ ] `Texture3DPingPong.create(device, 32)` returns distinct `read` / `write`; after `swap()`, former `write` is now `read`
- [ ] `destroy()` releases GPU memory without error
- [ ] `pnpm --filter belfast typecheck` and `pnpm --filter belfast build` pass

### 9.2 Consumer (fluid-sim-belfast)

- [ ] `FluidSimulation` uses `Texture3DPingPong` for velocity, density, pressure — no raw `device.gpu.createTexture` in the sim class
- [ ] Full `update()` cycle runs at `32³` without validation errors
- [ ] `fluid.velocity` and `fluid.density` expose `.view` and `.sampler` usable by arrow renderer
- [ ] Random `updateFlow` splats produce visible motion in `apps/fluid-sim-3d`

### 9.3 Documentation

- [ ] `docs/api/Texture3D.md` added (mirrors `Texture.md` structure)
- [ ] `docs/api/README.md` export table updated
- [ ] Optional: minimal `examples/volume-texture` or extend `particles-compute` with a 3D write + sample smoke test

---

## 10. Suggested implementation layout

```
packages/belfast/src/core/Texture3D.ts
packages/belfast/src/helper/Texture3DPingPong.ts   # or core/ if preferred
packages/belfast/src/index.ts                     # exports
docs/api/Texture3D.md
examples/volume-texture/                            # optional smoke test
```

**Export from `index.ts`:**

```ts
export { Texture3D, type Texture3DOptions } from "./core/Texture3D";
export { Texture3DPingPong, type Texture3DPingPongOptions } from "./helper/Texture3DPingPong";
```

---

## 11. Open questions

| #   | Question                                                           | Recommendation                                                                    |
| --- | ------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| Q1  | Should `storageView` be a separate view object or alias `view`?    | Hide behind API; create separate views if required by WebGPU validation           |
| Q2  | One shared sampler across velocity + density, or per-texture?      | Per-texture sampler (matches `Texture`); consumer may share if settings identical |
| Q3  | Should `Texture3DPingPong` live in `core/` or `helper/`?           | `helper/` — parallels Alfrid `FboPingPong` as composition over `Texture3D`        |
| Q4  | Feature-detect `rgba32float` 3D storage support at `Device` level? | Defer; document requirement; consumer calls `assertWebGPUSupport()`               |
| Q5  | Non-cubic volumes `[nx, ny, nz]` in v1?                            | Support in `create()` API, but fluid sim only needs cubic `32³` for acceptance    |

---

## 12. References

| Resource                   | Path                                              |
| -------------------------- | ------------------------------------------------- |
| 2D fluid sim (API to port) | `experiments2/apps/window/src/fluid-sim/index.js` |
| 2D fluid shaders           | `experiments2/apps/window/src/fluid-sim/shaders/` |
| Alfrid ping-pong           | `experiments2/libs/alfrid/helper/FboPingPong.js`  |
| Belfast 2D Texture         | `packages/belfast/src/core/Texture.ts`            |
| Belfast Compute example    | `examples/particles-compute/src/main.ts`          |
| 3D fluid sim plan          | `experiments2` — 3D Fluid Sim with Belfast plan   |

---

## 13. Revision history

| Date       | Change                                            |
| ---------- | ------------------------------------------------- |
| 2026-06-15 | Initial draft from fluid-sim-belfast requirements |
