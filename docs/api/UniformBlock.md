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
- `vec2f`
- `vec3f`
- `vec4f`
- `mat4x4f`

`UniformBlock` applies WGSL-style uniform alignment for these flat field types and writes into one contiguous `Float32Array`.

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
