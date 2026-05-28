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
| `slot`        | Vertex buffer slot (default: lowest unused slot)   |
| `stepMode`    | `"vertex"` (default) or `"instance"`               |

Returns `this` for chaining.

## Methods

### `getVertexLayouts(): (GPUVertexBufferLayout | null)[]`

Pass to `Draw` options as `vertexBuffers` when creating the pipeline. Unused slots between bindings are filled with `null` (required by WebGPU for non-contiguous vertex buffer indices).

### `bind(passEncoder)`

Calls `setVertexBuffer` for each binding. Used internally by `Draw.draw`.

### `setIndexBuffer(buffer, count, format?)`

Assigns index buffer and index metadata for indexed rendering.

| Argument | Description                                       |
| -------- | ------------------------------------------------- |
| `buffer` | Belfast `Buffer` created with `BufferUsage.index` |
| `count`  | Number of indices                                 |
| `format` | `"uint16"` (default) or `"uint32"`                |

When set, `Draw.draw(pass, mesh)` will use `drawIndexed(...)` automatically.

### `setIndexBufferFromData(device, indices, label?)`

Creates an index `Buffer` from `Uint16Array` or `Uint32Array`, infers index format, binds it to the mesh, and returns the created `Buffer`.

| Argument  | Description                                           |
| --------- | ----------------------------------------------------- |
| `device`  | Belfast `Device`                                      |
| `indices` | `Uint16Array` or `Uint32Array`                        |
| `label`   | Optional GPU buffer label (default: `"mesh-indices"`) |

Use this helper when you want one call for upload + bind:

```ts
const indexBuffer = mesh.setIndexBufferFromData(device, geom.indices, "cube-indices");
```

## Example

```ts
const mesh = new Mesh(3).addVertexBuffer({
  buffer: positionBuffer,
  arrayStride: 8,
  attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
});

mesh.setIndexBuffer(indexBuffer, indexCount, "uint16");
```

## See also

- [Draw](Draw.md) — `draw(pass, mesh)` or procedural `draw(pass, vertexCount)`
- [Buffer](Buffer.md)
