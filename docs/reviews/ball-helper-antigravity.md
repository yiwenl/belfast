# Antigravity review: BallHelper debug sphere

**Slug:** `ball-helper`  
**Reviewed:** 2026-05-24T18:00:00Z  
**Manifest:** `docs/reviews/queue/ball-helper.json`

## Summary

`BallHelper` is an outstanding, professional-grade implementation that perfectly matches Alfrid `DrawBall` functionality. The handling of alpha blending composites and explicitly setting `depthWriteEnabled: false` to avoid transparent sorting / z-fighting hazards is top-tier rendering engineering. Reusing the camera's `viewProj` layout (group 0) while allocating a separate instance uniform group (group 1) is beautifully elegant. With a crucial warning documented regarding multi-draw buffer overwrites, this is ready to merge.

## Critical

None.

## Suggestions

- **Document the WebGPU multi-draw buffer overwrite hazard**:
  Currently, `BallHelper.draw()` writes to its `instanceBuffer` per call using `queue.writeBuffer()` and then issues the draw command:

  ```ts
  writeInstanceUniform(this.instanceBuffer, this.device, params);
  this.meshDraw.draw(passEncoder, this.mesh, [sceneBindGroup, this.instanceBindGroup]);
  ```

  In WebGPU, `queue.writeBuffer` is queued immediately on the device queue, whereas `passEncoder.draw` commands are recorded and only executed later when `queue.submit([encoder.finish()])` is called.
  As a result, if a user attempts to draw multiple spheres in a single frame using the same `BallHelper` instance:

  ```ts
  ball.draw(pass, sceneBG, { position: [0, 0, 0] });
  ball.draw(pass, sceneBG, { position: [2, 0, 0] });
  ```

  Both `writeBuffer` calls will run first on the queue, meaning the second write will completely overwrite the first. When the GPU eventually executes the draws, **both spheres will be rendered at the second position (`[2, 0, 0]`)**.

  _Fix recommendation:_ Add a warning in `docs/api/BallHelper.md` explaining that a single `BallHelper` instance represents a single drawable sphere per frame. To render multiple spheres, developers must either instantiate multiple `BallHelper` instances, or we must implement dynamic uniform offsets / instanced drawing in a future follow-up.

- **Symmetric floating-point precision in `geom/sphere.ts`**:
  In the sphere geometry generation:
  ```ts
  x = Math.floor(x * precision) / precision;
  ```
  `Math.floor` is asymmetric for negative numbers (it rounds away from zero, whereas for positive numbers it rounds toward zero). This can lead to very tiny seam misalignments or asymmetric scaling along negative axes.
  _Recommendation:_ Use `Math.round()` instead of `Math.floor()` for symmetric rounding, or omit the rounding entirely since modern WebGPU handles floating-point precision of sphere geometries beautifully.

## Nits

- **Draw Order documentation**:
  The documentation in `docs/api/BallHelper.md` explaining that opaque geometry (triangle, axes) must be drawn _before_ the transparent sphere is exceptionally helpful and correct.

## Test plan gaps

None. The updated `camera-orbit` example successfully validates the alpha blending and depth-write disabling of the sphere, composite rendering it beautifully over the coordinate axes and 3D triangle.
