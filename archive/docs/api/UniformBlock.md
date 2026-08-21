# UniformBlock

Named uniform-buffer packing with explicit schema.

## Import

```ts
import {
  UniformBlock,
  type UniformBlockSchema,
  type UniformFieldType,
  Buffer,
  BufferUsage,
} from "belfast";
```

## Supported field types (v1)

- `f32`
- `u32`
- `vec2f`
- `vec3f`
- `vec4f`
- `mat4x4f`

`UniformBlock` applies WGSL-style uniform alignment for these flat field types and writes into one contiguous buffer. Float fields are exposed through the existing `Float32Array` view; `u32` fields write through an unsigned integer view over the same bytes.

## Create

```ts
const sceneUniforms = UniformBlock.create({
  viewProj: "mat4x4f",
  model: "mat4x4f",
  lightDir: "vec4f",
});
```

## Methods

### `set(name, value)`

Set a named field:

```ts
sceneUniforms.set("viewProj", camera.getViewProjectionMatrix());
sceneUniforms.set("model", modelMatrix);
sceneUniforms.set("lightDir", [-0.6, -0.7, -0.4, 0]);
```

Unsigned integer fields accept finite, non-negative integers in the `u32` range:

```ts
const simUniforms = UniformBlock.create({
  time: "f32",
  count: "u32",
});

simUniforms.set("time", 1.5).set("count", 200_000);
```

### `toFloat32Array()`

Returns the internal packed float data view.

### `data`

Read-only getter for the internal packed float data view.

### `writeToBuffer(buffer, device, byteOffset?)`

Writes packed data to a Belfast `Buffer` in one call.

### `getOffset(name)`

Returns packed float offset for a field (useful for debugging/interoperability).

## Size helpers

- `byteSize`: packed byte size of the schema
- `floatCount`: packed float count

Use with `Buffer.create`:

```ts
const uniformBuffer = Buffer.create(
  device,
  Buffer.uniformSize(sceneUniforms.byteSize),
  BufferUsage.uniform,
  "scene-uniforms",
);
```

## Notes

- Schema is explicit by design (safer than dynamic field inference).
- v1 supports flat fields only (no nested structs/arrays yet).
- `u32` values are intentionally rejected when fractional, negative, non-finite, or outside the unsigned 32-bit range.
