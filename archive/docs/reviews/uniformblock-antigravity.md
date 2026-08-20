# Antigravity review: UniformBlock named uniform packing

**Slug:** `uniformblock`  
**Reviewed:** 2026-05-28T11:49:00Z  
**Manifest:** `docs/reviews/queue/uniformblock.json`

## Summary

This is an exceptionally clean, well-thought-out, and valuable addition to Belfast. Designing the `UniformBlock` to enforce an explicit layout schema first is highly superior to dynamic field inference, ensuring predictable alignment, robust type checking, and keeping the public wrapper API state-free. The alignment mapping aligns perfectly with standard WGSL layout constraints (including padding `vec3f` up to 16 bytes).

However, a **critical allocation performance bottleneck** was found in the hot path of `set()`. Once this is resolved, this is highly recommended for immediate merge.

## Critical

- **Avoid Per-Frame GC Allocations in `UniformBlock.set()`**:
  Inside `UniformBlock.ts` line 104, values are set using:

  ```ts
  this.data.set(Array.from(value).slice(0, field.valueFloatCount), field.floatOffset);
  ```

  Since `set()` runs every single frame in the render loop for matrices and vectors (e.g. view-projection, model, lighting), `Array.from(value)` forces new array allocations, and `.slice()` makes another copy. This creates intense garbage collection pressure, leading to frame rate drops and micro-stuttering on complex scenes.

  _Fix recommendation:_ Use a zero-allocation typed array path via `.subarray()` if the value is already a `Float32Array`, and fall back to element-by-element iteration for other array-likes:

  ```ts
  if (value instanceof Float32Array) {
    this.data.set(value.subarray(0, field.valueFloatCount), field.floatOffset);
  } else {
    const len = field.valueFloatCount;
    const offset = field.floatOffset;
    for (let i = 0; i < len; i++) {
      this.data[offset + i] = value[i];
    }
  }
  ```

## Suggestions & Nits

- **Convenience `Float32Array` Getter**:
  Right now, you have `toFloat32Array()`. Consider also making a read-only getter `get data()` or exposing a clean property view if we want to align with other wrapper paradigms in the library, although the explicit method `toFloat32Array()` is perfectly fine.

- **Typescript Compilation / Build**:
  All builds, typechecks, and examples compile clean. The integration in `depth-to-texture/src/main.ts` looks highly professional and serves as a great demonstration.

## Test plan gaps

None. The schema layout constraints are fully validated and compile cleanly under the example target.
