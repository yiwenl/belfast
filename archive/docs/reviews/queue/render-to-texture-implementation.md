# Implementation: Render to texture

**Slug:** `render-to-texture`  
**Branch:** `refactor` (or current feature branch)  
**Status:** PENDING_REVIEW  
**Submitted:** 2026-05-28

## Summary

Adds a public render-to-texture workflow:

- `RenderTarget` core abstraction (offscreen color/depth framebuffer-style target)
- `CopyHelper` helper for fullscreen texture blit (alfrid `DrawCopy` style, helper naming aligned)
- New `render-to-texture` example that renders a lit cube (position + normal attributes) into an offscreen texture, then draws that texture to the screen.

## What changed

### Core API

| File                                                                                              | Change                                                                                                           |
| ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| [`packages/belfast/src/core/RenderTarget.ts`](../../../packages/belfast/src/core/RenderTarget.ts) | New offscreen render target API (`create`, `resize`, `beginRenderPass`, `destroy`)                               |
| [`packages/belfast/src/core/RenderPass.ts`](../../../packages/belfast/src/core/RenderPass.ts)     | `beginRenderPass` now accepts `GPUTextureView` or render-pass target object (`colorView` + optional `depthView`) |

### Helpers

| File                                                                                              | Change                                                          |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| [`packages/belfast/src/helper/CopyHelper.ts`](../../../packages/belfast/src/helper/CopyHelper.ts) | New fullscreen copy helper (`draw(pass, textureView, sampler)`) |

### Exports

| File                                                                      | Change                                                                          |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| [`packages/belfast/src/index.ts`](../../../packages/belfast/src/index.ts) | Export `RenderTarget`, `RenderTargetOptions`, `CopyHelper`, `CopyHelperOptions` |

### Example

| File                                                                                                                    | Purpose                        |
| ----------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| [`examples/render-to-texture/src/main.ts`](../../../examples/render-to-texture/src/main.ts)                             | Two-pass render pipeline       |
| [`examples/render-to-texture/src/shaders/cube-lit.wgsl`](../../../examples/render-to-texture/src/shaders/cube-lit.wgsl) | Simple diffuse-lit cube shader |
| [`examples/render-to-texture/`](../../../examples/render-to-texture/)                                                   | Full example scaffold          |

## Render flow

1. **Pass A (offscreen):**
   - Render lit cube to `RenderTarget` color texture
   - Depth attachment enabled (`withDepth: true`)
2. **Pass B (onscreen):**
   - Use `CopyHelper` to draw `RenderTarget.colorView` fullscreen to swapchain

## Design decisions

### 1) Public core abstraction first

`RenderTarget` lives in `core/` to be reusable beyond the example and avoid repeated ad-hoc depth/resize logic.

### 2) Helper naming consistency

Following current helper naming (`AxisHelper`, `BallHelper`), the copy utility is named **`CopyHelper`** (not `DrawCopy`).

### 3) Lit cube includes normals

Cube geometry includes a normal attribute per vertex (`@location(1)`), enabling basic diffuse lighting (`N dot L`) in WGSL.

### 4) No camera dependency in copy pass

`CopyHelper` uses a fullscreen triangle and texture sampling only; no mesh/camera uniforms required for the second pass.

### 5) Bind-group caching in `CopyHelper`

`CopyHelper.draw` now caches its internal bind group by `(textureView, sampler)` and only recreates when input bindings change. This avoids per-frame bind-group allocations in steady-state rendering.

## Post-review fixes (Antigravity)

- Cached `CopyHelper` bind group to avoid per-frame allocation/GC churn

## Verification

```bash
pnpm --filter belfast typecheck
pnpm --filter belfast build
pnpm --filter @belfast/example-render-to-texture typecheck
pnpm dev:example render-to-texture
```

Manual checks:

- Cube renders with diffuse lighting in offscreen pass
- Final screen displays offscreen texture via `CopyHelper`
- Orbit controls affect cube view in first pass
- Resize updates render target dimensions correctly

## Out of scope

- Mipmapped render targets
- Multiple color attachments / MRT
- Post-processing chain manager (single offscreen target only)
