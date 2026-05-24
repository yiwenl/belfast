# Draw

Builds a render pipeline from WGSL source and issues draw calls on a render pass encoder.

## Import

```ts
import { Draw, type DrawOptions } from "belfast";
import type { Mesh } from "belfast";
```

## Constructor

```ts
new Draw(device: Device, shaderCode: string, optionsOrLabel?: DrawOptions | string)
```

| Argument         | Description                                                               |
| ---------------- | ------------------------------------------------------------------------- |
| `device`         | Belfast `Device` (uses `device.device` and `device.format`)               |
| `shaderCode`     | Full WGSL source string (e.g. from `import shader from "./foo.wgsl?raw"`) |
| `optionsOrLabel` | Optional `DrawOptions` object or legacy label string                      |

### `DrawOptions`

| Field           | Type                      | Default                         | Description                           |
| --------------- | ------------------------- | ------------------------------- | ------------------------------------- |
| `label`         | `string`                  | `"Draw"`                        | Debug label prefix                    |
| `primitive`     | `GPUPrimitiveState`       | `{ topology: "triangle-list" }` | Primitive topology/culling/frontFace  |
| `depthStencil`  | `GPUDepthStencilState`    | `undefined`                     | Enable depth/stencil pipeline state   |
| `targets`       | `GPUColorTargetState[]`   | `[{ format: device.format }]`   | Color attachments for fragment output |
| `vertexBuffers` | `GPUVertexBufferLayout[]` | `[]`                            | From `mesh.getVertexLayouts()`        |

Creates:

- A `GPUShaderModule` with entry points `vs_main` and `fs_main`
- A `GPURenderPipeline` with `layout: "auto"`, vertex buffer layouts, and configurable state

## Methods

### `draw(passEncoder, mesh, instanceCount?)`

| Argument        | Default | Description                   |
| --------------- | ------- | ----------------------------- |
| `passEncoder`   | —       | Active `GPURenderPassEncoder` |
| `mesh`          | —       | `Mesh` with bound buffers     |
| `instanceCount` | `1`     | Instance count                |

Sets the pipeline, binds mesh vertex buffers, and draws `mesh.vertexCount` vertices.

## WGSL requirements

Your shader module must define vertex inputs matching `Mesh` attribute locations:

```wgsl
struct VertexInput {
  @location(0) position: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> { ... }
```

## Example

```ts
const mesh = new Mesh(3).addVertexBuffer({
  buffer: positionBuffer,
  arrayStride: 8,
  attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
});

const draw = new Draw(device, shaderCode, {
  label: "Triangle",
  vertexBuffers: mesh.getVertexLayouts(),
});

draw.draw(pass, mesh);
```

## See also

- [Mesh](Mesh.md) — vertex buffer bindings
- [Device](Device.md) — must be created first
- [RenderPass](RenderPass.md) — pass encoder for `draw()`
