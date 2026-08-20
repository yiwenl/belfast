# Antigravity review: Bind groups and uniforms

**Slug:** `bind-groups-uniforms`  
**Reviewed:** 2026-05-24T15:10:00Z  
**Manifest:** `docs/reviews/queue/bind-groups-uniforms.json`

## Summary

This is a beautiful, highly precise implementation of uniform buffers and bind groups in Belfast. Caching the `BindGroup` and only writing to the underlying `Buffer` per frame follows top-tier WebGPU performance recommendations. Incorporating the procedural `number` fallback in `Draw.draw()` and the sparse array `(GPUVertexBufferLayout | null)[]` typing in `DrawOptions` (from our previous review) were executed flawlessly. With some future-proofing for multi-resource bind groups and ergonomic signature polish, this is ready to merge.

## Critical

None.

## Suggestions

- **Future-proofing `BindGroup.create` for textures/samplers**:
  Currently, `BindGroup.create()` only supports a single uniform buffer binding. While perfect for the animated triangle example, this will immediately block texturing (which requires a `GPUTextureView` and a `GPUSampler` in the same bind group) or multiple uniform buffers.
  _Recommendation:_ In a follow-up, support an array of entries or resources while keeping a simple overload for the common single-buffer case:
  ```ts
  export interface BindGroupResource {
    binding: number;
    resource: GPUBindingResource | Buffer; // Auto-unpack Belfast Buffer to Buffer.gpu
  }
  ```
- **Ergonomics of `Draw.draw` parameters**:
  Right now, if a user wants to draw a mesh with a uniform bind group but without instancing (a very common scenario), they must write `draw(pass, mesh, 1, bindGroup)`. Passing `1` as a magic number for instancing is suboptimal for ergonomics.
  _Recommendation:_ Consider using method overloading or swap the parameter order:
  ```ts
  // Support both:
  draw(pass: GPURenderPassEncoder, mesh: Mesh | number, bindGroup?: BindGroup, instanceCount?: number): void;
  ```
- **Document Uniform Offset Alignment (256-byte limit)**:
  `Buffer.uniformSize()` is excellent for aligning individual structures to 16 bytes for WGSL struct layout matching. However, if Belfast users eventually use dynamic uniform buffer offsets (e.g. `setBindGroup` with offsets) or place multiple structures sequentially in one giant uniform buffer, WebGPU mandates that bind offsets align to the device's `minUniformBufferOffsetAlignment` (typically 256 bytes).
  _Recommendation:_ Add a note in `docs/api/Buffer.md` or `docs/api/BindGroup.md` warning developers that while struct layout size needs 16-byte alignment, buffer _binding offsets_ (when slicing or offset-binding) must align to 256 bytes.

## Nits

- **`BindGroup` missing a `destroy` or release method**:
  While the underlying uniform buffer is destroyed via `Buffer.destroy()`, there is no explicit cleanup needed for `GPUBindGroup` itself (it is garbage collected when no longer bound/referenced). However, adding a brief note in the documentation confirming this will help developers coming from native APIs like Vulkan/DirectX12.

## Test plan gaps

None. The animated `triangle-time` example is exceptionally well-structured, compiles successfully, uses auto-layouts correctly, and perfectly validates the new uniform buffer functionality.
