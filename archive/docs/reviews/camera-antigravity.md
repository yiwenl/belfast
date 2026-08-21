# Antigravity review: Camera (Perspective + Orthographic)

**Slug:** `camera`  
**Reviewed:** 2026-05-24T15:40:00Z  
**Manifest:** `docs/reviews/queue/camera.json`

## Summary

The camera implementation is extremely clean and elegantly integrates perspective/orthographic controls without introducing external runtime dependencies like `gl-matrix`. The `PerspectiveCamera` and its 3D depth-enabled frame loop in the `camera-triangle` demo are executed beautifully. However, there is a critical WebGPU-specific clipping hazard in the orthographic projection matrix math, along with a major documentation-to-code signature mismatch for `Draw.draw()` that needs immediate correction before merge.

## Critical

- **WebGPU Orthographic Z-Clip Bug in `mat4.ortho`**:
  In `packages/belfast/src/math/mat4.ts` (lines 143-166), the orthographic projection matrix uses standard OpenGL-style Z-mapping (`[-1, 1]` Z range) instead of WebGPU-style Z-mapping (`[0, 1]` Z range):

  ```ts
  out[10] = 2 * nf;
  out[14] = (far + near) * nf;
  ```

  In WebGPU, fragments with clip-space `z < 0` are discarded. Using an OpenGL projection matrix means that any geometry in the front half of the orthographic frustum (mapping to `[-1, 0]`) will be silently clipped.

  _Fix recommendation:_ Modify `mat4.ortho` to target the `[0, 1]` WebGPU clip-space Z range:

  ```ts
  export function ortho(
    out: Mat4,
    left: number,
    right: number,
    bottom: number,
    top: number,
    near: number,
    far: number,
  ): Mat4 {
    const lr = 1 / (left - right);
    const bt = 1 / (bottom - top);
    const nf = 1 / (near - far);

    out.fill(0);
    out[0] = -2 * lr;
    out[5] = -2 * bt;
    out[10] = nf; // WebGPU 0 to 1 range (instead of 2 * nf)
    out[12] = (left + right) * lr;
    out[13] = (top + bottom) * bt;
    out[14] = near * nf; // WebGPU near to far offset (instead of (far + near) * nf)
    out[15] = 1;

    return out;
  }
  ```

- **Documentation Mismatch for `Draw.draw` Signature**:
  The new API documentation in [`docs/api/Draw.md`](file:///Users/yi-wenlin/Development/belfast/docs/api/Draw.md) (lines 47-54) states the signature is:
  `draw(passEncoder, meshOrVertexCount, instanceCount?, bindGroup?)`
  And the documentation example calls:
  `draw.draw(pass, mesh, 1, bindGroup);`
  However, the actual code in `packages/belfast/src/helper/Draw.ts` (lines 51-56) implements:
  `draw(passEncoder, meshOrVertexCount, bindGroup?, instanceCount = 1)`
  And the `camera-triangle` demo calls `draw.draw(pass, mesh, bindGroup)`.

  _Fix recommendation:_ Update the API documentation in `docs/api/Draw.md` to match the actual code signature (placing `bindGroup` as the 3rd argument, which is highly preferred since instancing is rarely used compared to binding uniforms).

## Suggestions

- **Orthographic Parameter Consistency**:
  In `OrthographicCamera.ts`, the constructor and `setOrthographic` use the argument order:
  `constructor(left, right, top, bottom, near, far)`
  However, the internal `mat4.ortho` signature expects `bottom` before `top`:
  `ortho(out, left, right, bottom, top, near, far)`
  While the camera class handles this translation correctly under the hood, standard WebGPU and math libraries (like `gl-matrix`) typically maintain the consistent order of `left, right, bottom, top` across all constructors and math helpers. Consider aligning them to prevent developer confusion.

## Nits

- **Floating-point Aspect Precision in Demo**:
  In `examples/camera-triangle/src/main.ts` (line 98), calling `camera.setAspect(canvas.width / canvas.height)` inside the render loop works, but can be skipped unless the window actually resizes. Keeping it there is harmless, but moving it into a resize observer or window resize event handler is cleaner.

## Test plan gaps

- **Orthographic Camera Verification**:
  Because there is currently no example or test utilizing the `OrthographicCamera`, the OpenGL Z-clip bug went unnoticed. We strongly recommend adding a simple 2D or flat-shading orthographic scene example (e.g. `examples/ortho-sprite/` or modifying `examples/triangle` to support toggling ortho mode) to verify clip-space correctness.
