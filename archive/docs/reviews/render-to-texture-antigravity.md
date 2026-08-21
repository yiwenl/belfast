# Antigravity review: RenderTarget API, CopyHelper, and render-to-texture example

**Slug:** `render-to-texture`  
**Reviewed:** 2026-05-28T07:20:00Z  
**Manifest:** `docs/reviews/queue/render-to-texture.json`

## Summary

This is a stellar, high-quality implementation of offscreen rendering and texture blitting in Belfast. The clean `RenderTarget` abstraction lives perfectly in `core/` and integrates beautifully with existing `RenderPass` abstractions. The fullscreen triangle trick in `CopyHelper` is extremely efficient and elegant. The multi-pass render loop in the `render-to-texture` example is beautiful and serves as an exceptional demonstration. With one minor, crucial performance optimization to cache bind groups inside `CopyHelper`, this is ready to merge.

## Critical

None.

## Suggestions

- **Cache BindGroups inside `CopyHelper.draw` to prevent per-frame GC Churn**:
  Currently, `CopyHelper.draw` allocates a new `BindGroup` every frame inside its render loop:

  ```ts
  const bindGroup = BindGroup.create(
    this.device,
    this.bindGroupLayout,
    [
      { binding: 0, resource: textureView },
      { binding: 1, resource: sampler },
    ],
    "copy-helper-bind-group",
  );
  ```

  Since `CopyHelper` is typically invoked every frame to copy offscreen targets to the swapchain, creating a new `GPUBindGroup` and wrapping object every frame causes unnecessary garbage collection overhead and CPU cycles.

  _Fix recommendation:_ Cache the created bind group and only recreate it if the input `textureView` or `sampler` changes:

  ```ts
  export class CopyHelper {
    private readonly device: Device;
    private readonly drawPass: Draw;
    private readonly bindGroupLayout: GPUBindGroupLayout;

    private cachedTextureView?: GPUTextureView;
    private cachedSampler?: GPUSampler;
    private cachedBindGroup?: BindGroup;

    constructor(device: Device, options: CopyHelperOptions = {}) { ... }

    draw(
      passEncoder: GPURenderPassEncoder,
      textureView: GPUTextureView,
      sampler: GPUSampler,
      options: CopyDrawOptions = {},
    ): void {
      if (
        !this.cachedBindGroup ||
        this.cachedTextureView !== textureView ||
        this.cachedSampler !== sampler
      ) {
        this.cachedTextureView = textureView;
        this.cachedSampler = sampler;
        this.cachedBindGroup = BindGroup.create(
          this.device,
          this.bindGroupLayout,
          [
            { binding: 0, resource: textureView },
            { binding: 1, resource: sampler },
          ],
          "copy-helper-bind-group",
        );
      }
      // ...
      this.drawPass.draw(passEncoder, 3, this.cachedBindGroup);
    }
  }
  ```

  This keeps the copy blit 100% allocation-free during active frame rendering.

## Nits

- **Viewport and Scissor logic in `CopyHelper`**:
  Exposing `x, y, width, height` in `CopyDrawOptions` and using `passEncoder.setViewport` / `setScissorRect` to support drawing miniature preview overlays is exceptionally elegant and works perfectly.

## Test plan gaps

None. The `render-to-texture` example is a fantastic and comprehensive verification plan, showcasing offscreen rendering, viewport scissor overlays, and interactive camera orbits seamlessly.
