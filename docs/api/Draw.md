# Draw

Builds a render pipeline from WGSL source and issues draw calls on a render pass encoder.

## Import

```ts
import { Draw, type DrawOptions } from "belfast";
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

| Field          | Type                    | Default                         | Description                           |
| -------------- | ----------------------- | ------------------------------- | ------------------------------------- |
| `label`        | `string`                | `"Draw"`                        | Debug label prefix                    |
| `primitive`    | `GPUPrimitiveState`     | `{ topology: "triangle-list" }` | Primitive topology/culling/frontFace  |
| `depthStencil` | `GPUDepthStencilState`  | `undefined`                     | Enable depth/stencil pipeline state   |
| `targets`      | `GPUColorTargetState[]` | `[{ format: device.format }]`   | Color attachments for fragment output |

Creates:

- A `GPUShaderModule` with entry points `vs_main` and `fs_main`
- A `GPURenderPipeline` with `layout: "auto"` and configurable primitive/depth/target state

## Methods

### `draw(passEncoder, vertexCount?, instanceCount?)`

| Argument        | Default | Description                            |
| --------------- | ------- | -------------------------------------- |
| `passEncoder`   | —       | Active `GPURenderPassEncoder`          |
| `vertexCount`   | `3`     | Vertices to draw (3 = single triangle) |
| `instanceCount` | `1`     | Instance count                         |

Sets the pipeline and calls `passEncoder.draw(vertexCount, instanceCount)`.

## WGSL requirements

Your shader module must define:

```wgsl
@vertex
fn vs_main(...) -> @builtin(position) vec4<f32> { ... }

@fragment
fn fs_main(...) -> @location(0) vec4<f32> { ... }
```

The triangle example uses `@builtin(vertex_index)` and no vertex buffers.

## Example

```ts
import shaderCode from "./shaders/triangle.wgsl?raw";

const draw = new Draw(device, shaderCode, "Triangle");

// inside render loop, after beginRenderPass:
draw.draw(pass);
```

```ts
const drawWithDepth = new Draw(device, shaderCode, {
  label: "Mesh",
  primitive: { topology: "triangle-list", cullMode: "back" },
  depthStencil: {
    depthWriteEnabled: true,
    depthCompare: "less",
    format: "depth24plus",
  },
});
```

## See also

- [Device](Device.md) — must be created first
- [RenderPass](RenderPass.md) — pass encoder for `draw()`
