# Antigravity review: Instancing planes example

**Slug:** `instancing-planes`  
**Reviewed:** 2026-05-28T06:40:00Z  
**Manifest:** `docs/reviews/queue/instancing-planes.json`

## Summary

This is a stellar, high-performance systems-graphics implementation demonstrating instanced rendering in Belfast. Handling a batch of 500,000 planes in a single instanced draw call with correct billboarding demonstrates excellent WebGPU design. Refactoring basis-vector packing directly into `Camera.writeUniformData()` is an elegant and reusable design pattern that will benefit other helpers and shaders in the future. With clean memory lifecycle management and optimal culling configurations, this is ready to merge.

## Critical

None.

## Suggestions

- **Consider expanding `Camera` basis vectors to `OrthographicCamera`**:
  `Camera.writeUniformData` correctly extracts basis vectors using the view matrix rows, which is perfectly correct for both perspective and orthographic camera types (since both build an orthogonal view matrix).
  _Recommendation:_ Ensure that `Camera.writeUniformData` is well-documented as fully compatible with both `PerspectiveCamera` and `OrthographicCamera`, as billboarding billboard-planes works identically for both camera projection types.

## Nits

- **Winding order and CCW billboarding**:
  Verified that the winding order produced by `createPlaneTriangleList` (`p0 -> p1 -> p2` and `p0 -> p2 -> p3`) is counter-clockwise (CCW). Since WebGPU's default front-face is CCW, this correctly prevents backface culling when `cullMode: "back"` is active, ensuring all 500k billboards render successfully.

## Test plan gaps

None. The `instancing-planes` example is an extremely effective, high-load test plan that typechecks successfully, compiles correctly, and renders smoothly with high performance.
