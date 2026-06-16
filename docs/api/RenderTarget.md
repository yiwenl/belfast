# RenderTarget

Offscreen color/depth render target (framebuffer-style) for render-to-texture workflows.

## Import

```ts
import { RenderTarget, type RenderTargetOptions } from "belfast";
```

## Constructor

```ts
RenderTarget.create(device: Device, options: RenderTargetOptions)
```

### `RenderTargetOptions`

| Field               | Default                                           | Description               |
| ------------------- | ------------------------------------------------- | ------------------------- |
| `width`             | required                                          | Target width in pixels    |
| `height`            | required                                          | Target height in pixels   |
| `label`             | `"RenderTarget"`                                  | Debug label prefix        |
| `format`            | `device.format` (`rgba16float` when `device.hdr`) | Offscreen color format    |
| `withDepth`         | `false`                                           | Allocate depth attachment |
| `depthFormat`       | `"depth24plus"`                                   | Depth format when enabled |
| `depthTextureUsage` | `GPUTextureUsage.RENDER_ATTACHMENT`               | Depth texture usage       |

## Properties

| Property       | Type                            | Description                              |
| -------------- | ------------------------------- | ---------------------------------------- |
| `width`        | `number`                        | Current width                            |
| `height`       | `number`                        | Current height                           |
| `format`       | `GPUTextureFormat`              | Color format                             |
| `depthFormat`  | `GPUTextureFormat \| undefined` | Depth format                             |
| `colorView`    | `GPUTextureView`                | Color attachment / sampled view          |
| `depthView`    | `GPUTextureView \| undefined`   | Depth attachment view                    |
| `depthTexture` | `GPUTexture \| undefined`       | Depth texture                            |
| `sampler`      | `GPUSampler`                    | Default sampler for sampling `colorView` |

## Methods

### `resize(width, height)`

Recreates internal textures when size changes.

### `beginRenderPass(encoder, options?)`

Starts a render pass targeting this offscreen target.

Uses `beginRenderPass(...)` under the hood and auto-wires depth attachment when `withDepth` is enabled.

### `destroy()`

Destroys owned GPU textures.

## Example

```ts
const target = RenderTarget.create(device, {
  width: canvas.width,
  height: canvas.height,
  withDepth: true,
});

const passA = target.beginRenderPass(encoder);
// draw offscreen...
passA.end();
```

Use `target.colorView` and `target.sampler` in a later pass (for example with `CopyHelper`).

When `Device.create(..., { hdr: true })` is used and `format` is omitted, `RenderTarget` defaults to `rgba16float` to support 16-bit HDR rendering pipelines.
