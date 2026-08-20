# AxisHelper

RGB debug axes along world X (red), Y (green), and Z (blue). Built from six vertices and `line-list` topology (matching alfrid `DrawAxis`).

## Import

```ts
import { AxisHelper, type AxisHelperOptions } from "belfast";
```

## Constructor

```ts
new AxisHelper(device: Device, options?: AxisHelperOptions)
```

| Option           | Default        | Description                                                                                            |
| ---------------- | -------------- | ------------------------------------------------------------------------------------------------------ |
| `length`         | `1000`         | Half-extent of each axis (± on axis)                                                                   |
| `label`          | `"AxisHelper"` | Debug labels for GPU objects                                                                           |
| `pipelineLayout` | auto-created   | Pass `createSceneUniformPipelineLayout().pipelineLayout` when sharing a bind group with another `Draw` |

Creates position/color vertex buffers, a `Mesh` with 6 vertices, and an internal `Draw` pipeline with depth testing enabled.

## Methods

| Method                       | Description                                                    |
| ---------------------------- | -------------------------------------------------------------- |
| `draw(pass, bindGroup)`      | Draw axes in a render pass                                     |
| `destroy()`                  | Release GPU vertex buffers                                     |
| `getBindGroupLayout(index?)` | Layout for `SceneUniforms.viewProj` at `@group(0) @binding(0)` |

## Usage

Use the same `mat4x4` view-projection uniform as other camera-lit draws (`Buffer.uniformSize(64)`).

With `layout: "auto"`, each `Draw` pipeline gets its own bind group layout — **you cannot reuse one bind group** across `AxisHelper` and another `Draw`. Use an explicit shared layout:

```ts
const { pipelineLayout, bindGroupLayout } = createSceneUniformPipelineLayout(device);
const axes = new AxisHelper(device, { pipelineLayout });
const draw = new Draw(device, shaderCode, { layout: pipelineLayout, vertexBuffers: mesh.getVertexLayouts(), ... });
const bindGroup = BindGroup.create(device, bindGroupLayout, uniformBuffer);

uniformBuffer.write(device, camera.getViewProjectionMatrix());
axes.draw(pass, bindGroup);
draw.draw(pass, mesh, bindGroup);
```

See [camera-orbit example](../../examples/camera-orbit/src/main.ts).
