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

| Field           | Type                                | Default                         | Description                                                                                       |
| --------------- | ----------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------- |
| `label`         | `string`                            | `"Draw"`                        | Debug label prefix                                                                                |
| `layout`        | `GPUPipelineLayout \| "auto"`       | `"auto"`                        | Pipeline layout; use a shared layout from `createSceneUniformPipelineLayout` to reuse bind groups |
| `primitive`     | `GPUPrimitiveState`                 | `{ topology: "triangle-list" }` | Primitive topology/culling/frontFace                                                              |
| `depthStencil`  | `GPUDepthStencilState`              | `undefined`                     | Enable depth/stencil pipeline state                                                               |
| `targets`       | `GPUColorTargetState[]`             | `[{ format: device.format }]`   | Color attachments for fragment output                                                             |
| `vertexBuffers` | `(GPUVertexBufferLayout \| null)[]` | `[]`                            | From `mesh.getVertexLayouts()`                                                                    |

Creates:

- A `GPUShaderModule` with entry points `vs_main` and `fs_main`
- A `GPURenderPipeline` with `layout` (default `"auto"`), vertex buffer layouts, and configurable state

## Methods

### `getBindGroupLayout(index?)`

Returns `pipeline.getBindGroupLayout(index)` for creating bind groups after the pipeline exists. Default `index` is `0`.

Use with `BindGroup.create(device, layout, uniformBuffer)` when the shader declares `@group(0) @binding(0) var<uniform> ...`.

### `draw(passEncoder, meshOrVertexCount, bindGroup?, instanceCount?)`

| Argument            | Default | Description                                           |
| ------------------- | ------- | ----------------------------------------------------- |
| `passEncoder`       | —       | Active `GPURenderPassEncoder`                         |
| `meshOrVertexCount` | —       | `Mesh` (binds buffers) or `number` (procedural draw)  |
| `bindGroup`         | —       | Optional `BindGroup` or array (bound at indices 0..n) |
| `instanceCount`     | `1`     | Instance count                                        |

With a `Mesh`, sets the pipeline and bind groups, binds vertex buffers, and:

- calls `drawIndexed(...)` if `mesh.setIndexBuffer(...)` was used
- otherwise calls `draw(...)` with `mesh.vertexCount`

With a `number`, sets the pipeline, optional bind group, and draws that many vertices (no vertex buffer bind). Use when the vertex shader uses `@builtin(vertex_index)` and `vertexBuffers` was empty at pipeline creation.

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

With uniforms:

```ts
const bindGroup = BindGroup.create(device, draw.getBindGroupLayout(0), uniformBuffer);
uniformBuffer.write(device, new Float32Array([time, 0, 0, 0]));
draw.draw(pass, mesh, bindGroup);
```

## See also

- [BindGroup](BindGroup.md) — uniform bind groups
- [Mesh](Mesh.md) — vertex buffer bindings
- [Device](Device.md) — must be created first
- [RenderPass](RenderPass.md) — pass encoder for `draw()`
