# ShadowMap

Helper for quickly creating shadow maps with a depth texture and comparison sampler.

## Import

```ts
import { ShadowMap, type ShadowMapOptions } from "belfast";
```

## Constructor

```ts
ShadowMap.create(device: Device, options?: ShadowMapOptions)
```

### `ShadowMapOptions`

| Field    | Default          | Description                             |
| -------- | ---------------- | --------------------------------------- |
| `size`   | `1024`           | Resolution for the square depth texture |
| `format` | `"depth32float"` | Shadow map depth format                 |
| `label`  | `"ShadowMap"`    | Debug label prefix                      |

## Properties

| Property  | Type               | Description        |
| --------- | ------------------ | ------------------ |
| `texture` | `GPUTexture`       | Depth texture      |
| `view`    | `GPUTextureView`   | Depth texture view |
| `sampler` | `GPUSampler`       | Comparison sampler |
| `size`    | `[number, number]` | Size of shadow map |

## Methods

### `beginRenderPass(encoder, options?)`

Starts a depth-only render pass targeting this shadow map.

### `destroy()`

Destroys underlying textures and render targets.
