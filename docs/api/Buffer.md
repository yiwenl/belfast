# Buffer

Shareable GPU buffer wrapper with typed usage presets.

## Import

```ts
import { Buffer, BufferUsage } from "belfast";
```

## `BufferUsage`

| Preset          | Flags                           | Typical use                             |
| --------------- | ------------------------------- | --------------------------------------- |
| `vertex`        | `VERTEX \| COPY_DST`            | Static or CPU-updated vertex data       |
| `storage`       | `STORAGE \| COPY_DST`           | Compute read/write                      |
| `vertexStorage` | `VERTEX \| STORAGE \| COPY_DST` | Compute output consumed as vertex input |

## Static methods

### `Buffer.create(device, size, usage, label?)`

Allocates a `GPUBuffer` without initial data.

### `Buffer.fromData(device, data, usage, label?)`

Creates a buffer sized to `data` and uploads contents via `write`.

## Instance

| Property | Type                  |
| -------- | --------------------- |
| `gpu`    | `GPUBuffer`           |
| `size`   | `number`              |
| `usage`  | `GPUBufferUsageFlags` |
| `label`  | `string` (optional)   |

### `write(device, data, byteOffset?)`

Uploads CPU data without allocating a slice (uses native `queue.writeBuffer` offsets).

### `destroy()`

Destroys the underlying `GPUBuffer`.

## See also

- [Mesh](Mesh.md) — binds buffers into render passes
- [Feature: vertex buffers](../features/vertex-buffers.md)
