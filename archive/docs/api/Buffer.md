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
| `index`         | `INDEX \| COPY_DST`             | Index data for indexed rendering        |
| `storage`       | `STORAGE \| COPY_DST`           | Compute read/write                      |
| `uniform`       | `UNIFORM \| COPY_DST`           | Shader uniform data                     |
| `vertexStorage` | `VERTEX \| STORAGE \| COPY_DST` | Compute output consumed as vertex input |

## Static methods

### `Buffer.uniformSize(byteLength)`

Rounds `byteLength` up to a multiple of 16 (WGSL uniform struct alignment). Use when sizing uniform buffers that hold a single struct.

### Uniform binding offsets

`uniformSize()` does **not** apply to bind offsets. When using dynamic offsets in `setBindGroup`, or packing multiple uniform structs into one buffer, each offset must align to `device.device.limits.minUniformBufferOffsetAlignment` (typically **256 bytes**). See [BindGroup](BindGroup.md).

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
- [BindGroup](BindGroup.md) — uniform buffer bind groups
- [Feature: vertex buffers](../features/vertex-buffers.md)
