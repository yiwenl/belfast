# RenderPass

Helper to begin a single color render pass targeting the swapchain (or any `GPUTextureView`).

## Import

```ts
import { beginRenderPass, type RenderPassOptions } from "belfast";
```

## `beginRenderPass(encoder, view, options?)`

Returns a `GPURenderPassEncoder`. You must call `.end()` on it before submitting the command buffer.

```ts
const encoder = device.device.createCommandEncoder();
const view = device.getCurrentTexture().createView();
const pass = beginRenderPass(encoder, view);

// draw...

pass.end();
device.device.queue.submit([encoder.finish()]);
```

## `RenderPassOptions`

| Field                    | Type                                  | Default                               | Description                            |
| ------------------------ | ------------------------------------- | ------------------------------------- | -------------------------------------- |
| `clearColor`             | `GPUColor`                            | `{ r: 0.05, g: 0.05, b: 0.08, a: 1 }` | Clear color when `loadOp` is `"clear"` |
| `loadOp`                 | `GPULoadOp`                           | `"clear"`                             | Color attachment load operation        |
| `storeOp`                | `GPUStoreOp`                          | `"store"`                             | Color attachment store operation       |
| `depthStencilAttachment` | `GPURenderPassDepthStencilAttachment` | `undefined`                           | Optional depth/stencil attachment      |

One color attachment is always configured (the provided `view`). Depth is optional via `depthStencilAttachment`.

## Depth example

```ts
const depthTexture = device.device.createTexture({
  size: [canvas.width, canvas.height],
  format: "depth24plus",
  usage: GPUTextureUsage.RENDER_ATTACHMENT,
});

const pass = beginRenderPass(encoder, colorView, {
  depthStencilAttachment: {
    view: depthTexture.createView(),
    depthLoadOp: "clear",
    depthClearValue: 1.0,
    depthStoreOp: "store",
  },
});
```

## See also

- [Draw](Draw.md) — draw into this pass
- [Device](Device.md) — `getCurrentTexture()` for `view`
