# API reference

Public exports from [`packages/belfast/src/index.ts`](../../packages/belfast/src/index.ts).

```ts
import {
  BindGroup,
  Buffer,
  Device,
  Draw,
  Mesh,
  beginRenderPass,
  assertWebGPUSupport,
  showWebGPUUnavailableMessage,
} from "belfast";
```

## Modules

| Export                         | Kind     | Doc                            |
| ------------------------------ | -------- | ------------------------------ |
| `Device`                       | class    | [Device.md](Device.md)         |
| `DeviceOptions`                | type     | [Device.md](Device.md)         |
| `Buffer`                       | class    | [Buffer.md](Buffer.md)         |
| `BufferUsage`                  | const    | [Buffer.md](Buffer.md)         |
| `BindGroup`                    | class    | [BindGroup.md](BindGroup.md)   |
| `BindGroupResource`            | type     | [BindGroup.md](BindGroup.md)   |
| `Mesh`                         | class    | [Mesh.md](Mesh.md)             |
| `VertexBufferBinding`          | type     | [Mesh.md](Mesh.md)             |
| `VertexAttributeDescriptor`    | type     | [Mesh.md](Mesh.md)             |
| `Draw`                         | class    | [Draw.md](Draw.md)             |
| `DrawOptions`                  | type     | [Draw.md](Draw.md)             |
| `beginRenderPass`              | function | [RenderPass.md](RenderPass.md) |
| `RenderPassOptions`            | type     | [RenderPass.md](RenderPass.md) |
| `assertWebGPUSupport`          | function | [utilities.md](utilities.md)   |
| `showWebGPUUnavailableMessage` | function | [utilities.md](utilities.md)   |

## Versioning

The package is early (`0.1.x`). Breaking changes are expected until the API stabilizes.
