# Device

Entry point for WebGPU context setup. Wraps a canvas, `GPUCanvasContext`, and `GPUDevice`.

## Import

```ts
import { Device, type DeviceOptions } from "belfast";
```

## `DeviceOptions`

| Field             | Type                       | Default                               | Description                                                            |
| ----------------- | -------------------------- | ------------------------------------- | ---------------------------------------------------------------------- |
| `powerPreference` | `GPUPowerPreference`       | adapter default                       | Passed to `requestAdapter`                                             |
| `alpha`           | `boolean`                  | `true` (premultiplied)                | Set `false` for opaque canvas (`alphaMode: "opaque"`)                  |
| `hdr`             | `boolean`                  | `false`                               | Enables HDR defaults (`rgba16float` swapchain + extended tone mapping) |
| `colorSpace`      | `PredefinedColorSpace`     | `srgb`                                | Presentation color space                                               |
| `toneMappingMode` | `GPUCanvasToneMappingMode` | `standard` (or `extended` when `hdr`) | Canvas tone mapping mode                                               |

## Static methods

### `Device.isSupported(): Promise<boolean>`

Returns `false` if `navigator.gpu` is missing or no adapter can be requested. Does not create a device.

### `Device.create(canvas, options?): Promise<Device>`

1. Requests an adapter and device
2. Gets the `webgpu` canvas context
3. Configures the context with the preferred swapchain format

Throws if WebGPU is unavailable, adapter request fails, or the canvas context cannot be created.

```ts
const canvas = document.createElement("canvas");
document.body.appendChild(canvas);
const device = await Device.create(canvas);
```

HDR-oriented setup:

```ts
const device = await Device.create(canvas, { hdr: true });
// rgba16float swapchain, colorSpace: "srgb", toneMappingMode: "extended"
```

## Instance properties

| Property          | Type                       | Description                          |
| ----------------- | -------------------------- | ------------------------------------ |
| `canvas`          | `HTMLCanvasElement`        | The canvas passed to `create`        |
| `context`         | `GPUCanvasContext`         | WebGPU canvas context                |
| `device`          | `GPUDevice`                | Use for encoders, pipelines, buffers |
| `format`          | `GPUTextureFormat`         | Swapchain format (used by `Draw`)    |
| `colorSpace`      | `PredefinedColorSpace`     | Configured canvas color space        |
| `toneMappingMode` | `GPUCanvasToneMappingMode` | Configured canvas tone mapping mode  |
| `hdr`             | `boolean`                  | Whether HDR defaults were enabled    |

## Instance methods

### `resize(width?, height?)`

Sets `canvas.width` / `canvas.height` from arguments or `clientWidth` / `clientHeight`. Call once per frame (or on resize) before `getCurrentTexture()`.

### `getCurrentTexture(): GPUTexture`

Returns the current swapchain texture. Create a view for the render pass:

```ts
const view = device.getCurrentTexture().createView();
```

### `destroy()`

Calls `device.destroy()`. Use when tearing down the app.

## See also

- [Overview](../overview.md) — full render loop
- [Draw](Draw.md) — uses `device.format` for the pipeline
