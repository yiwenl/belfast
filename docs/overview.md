# Overview

Belfast is a thin wrapper around the WebGPU API. It does not hide the GPU — you still create command encoders, submit work to the queue, and write WGSL shaders. The library reduces boilerplate for common setup steps.

## Mental model

```mermaid
sequenceDiagram
  participant App
  participant Device
  participant Draw
  participant GPU

  App->>Device: create(canvas)
  App->>Draw: new Draw(device, wgsl)
  loop each frame
    App->>Device: resize()
    App->>GPU: createCommandEncoder()
    App->>Device: getCurrentTexture()
    App->>GPU: beginRenderPass(encoder, view)
    App->>Draw: draw(pass)
    App->>GPU: pass.end(), queue.submit()
  end
```

| Step   | Belfast API              | WebGPU underneath               |
| ------ | ------------------------ | ------------------------------- |
| Setup  | `Device.create()`        | adapter, device, canvas context |
| Buffer | `Buffer.fromData()`      | vertex data on GPU              |
| Mesh   | `Mesh.addVertexBuffer()` | layouts + per-pass bind         |
| Shader | `new Draw(device, wgsl)` | shader module + render pipeline |
| Frame  | `beginRenderPass()`      | render pass encoder             |
| Draw   | `draw.draw(pass, mesh)`  | `setPipeline` + `draw()`        |

## Minimal render loop

See the [triangle example](../examples/triangle/src/main.ts):

1. `assertWebGPUSupport()` — fail fast if WebGPU is unavailable
2. `Device.create(canvas)` — configure the swapchain
3. `Buffer.fromData` + `Mesh.addVertexBuffer` — upload positions, describe `@location(0)`
4. `new Draw(device, shaderWgsl, { vertexBuffers })` — compile WGSL + vertex layouts
5. Each frame:
   - `device.resize()` — match canvas pixel size
   - `device.getCurrentTexture().createView()` — swapchain texture
   - `beginRenderPass(encoder, view)` — clear and draw to screen
   - `draw.draw(pass, mesh)` — bind buffers and draw
   - `pass.end()` and `queue.submit([encoder.finish()])`

You can optionally pass `DrawOptions` to configure pipeline state (for example culling, color targets, and depth-stencil state).

## WGSL conventions

`Draw` expects a single WGSL module with:

- Vertex entry point: `vs_main`
- Fragment entry point: `fs_main`
- Fragment output matching the swapchain `format` from `device.format`

Vertex attributes use `@location(N)` in WGSL, matching `Mesh` attribute `shaderLocation` values.

## Depth-ready path

`beginRenderPass` accepts an optional `depthStencilAttachment`, so adding depth testing is a straight extension of the same frame loop:

- Create a depth texture (`format: "depth24plus"`, `usage: RENDER_ATTACHMENT`)
- Pass its view in `beginRenderPass(..., { depthStencilAttachment })`
- Create `Draw` with `depthStencil` options enabled

## What is not in the public API yet

These exist internally or are planned; they are not exported from `belfast` today:

- Cameras and scene graph
- Index buffers / `drawIndexed`
- Bind groups and uniforms
- Loaders, math utilities

When adding features, update [`api/README.md`](api/README.md) and add a focused page under `docs/api/`.
