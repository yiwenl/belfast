# Mesh

Vertex layout container: vertex count, buffer bindings, pipeline layouts, and pass binding.

**Naming:** Belfast uses `Mesh` for geometry data only (buffers + attributes). Shaders and pipeline state live on [`Draw`](Draw.md). A future `Geometry` type may appear if we split those roles more explicitly.

## Import

```ts
import { Mesh, type VertexBufferBinding, type VertexAttributeDescriptor } from "belfast";
```

## Constructor

```ts
new Mesh(vertexCount: number)
```

## `addVertexBuffer(binding)`

| Field         | Description                                        |
| ------------- | -------------------------------------------------- |
| `buffer`      | Belfast `Buffer`                                   |
| `arrayStride` | Bytes between consecutive vertices                 |
| `attributes`  | `shaderLocation`, `format`, `offset` per attribute |
| `slot`        | Vertex buffer slot (default: next free slot)       |
| `stepMode`    | `"vertex"` (default) or `"instance"`               |

Returns `this` for chaining.

## Methods

### `getVertexLayouts(): GPUVertexBufferLayout[]`

Pass to `Draw` options as `vertexBuffers` when creating the pipeline.

### `bind(passEncoder)`

Calls `setVertexBuffer` for each binding. Used internally by `Draw.draw`.

## Example

```ts
const mesh = new Mesh(3).addVertexBuffer({
  buffer: positionBuffer,
  arrayStride: 8,
  attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
});
```

## See also

- [Draw](Draw.md) — `draw(pass, mesh)`
- [Buffer](Buffer.md)
