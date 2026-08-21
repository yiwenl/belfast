# CopyHelper

Fullscreen texture blit helper (copy pass) similar to alfrid `DrawCopy`.

## Import

```ts
import { CopyHelper, type CopyHelperOptions } from "belfast";
```

## Constructor

```ts
new CopyHelper(device: Device, options?: CopyHelperOptions)
```

### `CopyHelperOptions`

| Field     | Default                       | Description            |
| --------- | ----------------------------- | ---------------------- |
| `label`   | `"CopyHelper"`                | Debug label prefix     |
| `targets` | `[{ format: device.format }]` | Pipeline color targets |

## Methods

### `draw(passEncoder, textureView, sampler)`

Draws the given texture to the full viewport using a fullscreen triangle.

No camera or mesh setup is needed in the app code.

## Example

```ts
const copy = new CopyHelper(device);
const screenPass = beginRenderPass(encoder, device.getCurrentTexture().createView());
copy.draw(screenPass, offscreen.colorView, offscreen.sampler);
screenPass.end();
```
