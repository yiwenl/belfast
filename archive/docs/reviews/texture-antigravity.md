# Antigravity review: Texture loading and texture example

**Slug:** `texture`  
**Reviewed:** 2026-05-24T18:20:00Z  
**Manifest:** `docs/reviews/queue/texture.json`

## Summary

This is an exceptionally high-quality PR that implements clean, flexible WebGPU texture loading and sampling. The introduction of the `BindGroupResource` array format and multiple overloads for `BindGroup.create()` (which perfectly implements our previous review suggestion!) is outstanding and significantly increases the library's versatility. Reusing the camera uniform binding at `@group(0) @binding(0)` so textured planes and `AxisHelper` can share a single bind group is a highly performant and elegant design. With one minor, crucial performance stall resolved in the load path, this is ready to merge.

## Critical

None.

## Suggestions

- **Remove the synchronous `onSubmittedWorkDone()` CPU-GPU stall**:
  In `packages/belfast/src/core/Texture.ts` (line 84), the `fromBitmap` method awaits a GPU completion signal:
  ```ts
  device.device.queue.copyExternalImageToTexture({ source: bitmap, flipY }, { texture: gpu }, [
    width,
    height,
  ]);
  await device.device.queue.onSubmittedWorkDone(); // <-- CPU Stall!
  ```
  WebGPU executes all queue operations sequentially and in order. Any subsequent command buffer submissions (such as frame renders referencing the texture view) will naturally wait until the queued `copyExternalImageToTexture` image upload completes on the GPU.
  Forcing the CPU to `await` `onSubmittedWorkDone()` creates a synchronous stall, blocking the main Javascript thread until the GPU is completely idle. If textures are loaded dynamically during gameplay or active renders, this will cause a highly visible stutter/hiccup.
  _Recommendation:_ Safely delete `await device.device.queue.onSubmittedWorkDone();` to make texture uploads completely non-blocking and asynchronous.

## Nits

- **`image.jpg` Aspect Ratio Logic**:
  The aspect ratio math in `examples/texture/src/main.ts` is perfectly calculated and correctly prevents texture stretching on vertical planes.
- **Vite Externalization**:
  Verified that rollup configurations properly bundle internal helper geometries while externalizing runtime library imports.

## Test plan gaps

None. The `texture` demo app builds cleanly, launches Vite with hot-reloads, and perfectly verifies the textured plane rendering under orbit controls.
