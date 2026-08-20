# Texture

Loads a 2D image into a `GPUTexture` with a view and sampler for WGSL `textureSample`.

## Import

```ts
import { Texture, type TextureOptions } from "belfast";
```

## Static methods

### `Texture.load(device, url, options?)`

Fetches an image URL, decodes to `ImageBitmap`, uploads to the GPU. Throws if the request fails.

```ts
const texture = await Texture.load(device, "/image.jpg");
```

### `Texture.fromBitmap(device, bitmap, options?)`

Uploads an existing `ImageBitmap` via a queued `copyExternalImageToTexture` (non-blocking). The GPU finishes the copy before any later queue work (e.g. the next frame render). Safe to `bitmap.close()` after this returns.

## `TextureOptions`

| Option         | Default           | Description                                      |
| -------------- | ----------------- | ------------------------------------------------ |
| `label`        | `"Texture"`       | GPU debug labels                                 |
| `format`       | `"rgba8unorm"`    | Texture format                                   |
| `flipY`        | `true`            | Flip vertically on upload (browser image origin) |
| `addressModeU` | `"clamp-to-edge"` | Sampler wrap U                                   |
| `addressModeV` | `"clamp-to-edge"` | Sampler wrap V                                   |
| `magFilter`    | `"linear"`        | Magnification filter                             |
| `minFilter`    | `"linear"`        | Minification filter                              |

## Instance

| Member    | Type             | Description    |
| --------- | ---------------- | -------------- |
| `width`   | `number`         | Pixel width    |
| `height`  | `number`         | Pixel height   |
| `view`    | `GPUTextureView` | Shader binding |
| `sampler` | `GPUSampler`     | Shader binding |

### `destroy()`

Destroys the `GPUTexture`. Samplers do not require explicit destruction.

## Bind group usage

Use with `createSceneTexturePipelineLayout` and `BindGroup.create`:

```ts
const { pipelineLayout, bindGroupLayout } = createSceneTexturePipelineLayout(device);

const bindGroup = BindGroup.create(device, bindGroupLayout, [
  { binding: 0, resource: uniformBuffer },
  { binding: 1, resource: texture.view },
  { binding: 2, resource: texture.sampler },
]);
```

WGSL:

```wgsl
@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var colorMap: texture_2d<f32>;
@group(0) @binding(2) var colorSampler: sampler;
```

`AxisHelper` can reuse the same bind group (it only samples binding 0).

See [texture example](../../examples/texture/src/main.ts).
