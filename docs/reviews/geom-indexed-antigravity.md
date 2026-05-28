# Antigravity review: Geom utility and indexed rendering support

**Slug:** `geom-indexed`  
**Reviewed:** 2026-05-28T08:10:00Z  
**Manifest:** `docs/reviews/queue/geom-indexed.json`

## Summary

This is an exceptionally high-quality PR that beautifully integrates indexed rendering support into Belfast's core API while introducing standard geometry generators (`Geom.plane`, `Geom.sphere`, and `Geom.cube`). Having `Draw.draw()` automatically detect the presence of an index buffer on the mesh and choose the correct WebGPU draw command under the hood (`passEncoder.draw` vs `passEncoder.drawIndexed`) is a brilliant and highly ergonomic design. The code quality, API cleanliness, and documentation updates are all top-tier.

## Critical

None.

## Suggestions

- **Consider helper methods for indexing buffers**:
  While `Mesh.setIndexBuffer` is clean, creating index buffers in WebGPU requires specifying both `BufferUsage.index` and the correct TypedArray data types (`Uint16Array` or `Uint32Array`).
  _Recommendation:_ To make the API even more streamlined and prevent developer mistakes, you could consider adding a shortcut helper directly to `Mesh` in the future:

  ```ts
  mesh.setIndexBufferFromData(device, geom.indices);
  ```

  Inside, this method would automatically check if the passed array is `Uint16Array` or `Uint32Array` (deducing the index format), allocate the buffer using `BufferUsage.index`, and bind it. This would hide some low-level WebGPU buffer management boilerplate from the user.

- **Auto-upgrading uint16 / uint32 in `Geom`**:
  Your automated index size upgrader:
  ```ts
  function toIndexArray(indices: number[], vertexCount: number): Uint16Array | Uint32Array {
    return vertexCount > 65535 ? new Uint32Array(indices) : new Uint16Array(indices);
  }
  ```
  is extremely robust and follows WebGPU best practices, protecting developers from the strict 16-bit indexing vertex limit.

## Nits

- **Refactoring render-to-texture**:
  The refactoring of the `render-to-texture` example to consume the new shared `Geom.cube()` primitive generator is highly appreciated and greatly reduces code duplication in the codebase.

## Test plan gaps

None. The new `geom-indexed` example is exceptionally well-structured, compiles successfully, typechecks with no errors, and renders the diffuse-lit indexed cube smoothly under orbital controls.
