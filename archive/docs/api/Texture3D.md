# Texture3D

Creates an empty 3D GPU volume with views for `texture_3d` sampling and `texture_storage_3d` compute writes.

## Import

```ts
import { Texture3D, type Texture3DOptions } from "belfast";
```

## Static methods

### `Texture3D.create(device, size, options?)`

Allocates a volumetric texture on the GPU. Does not upload initial data (starts zero/uninitialized).

```ts
const volume = Texture3D.create(device, 32);
const grid = Texture3D.create(device, [64, 32, 32], { label: "Velocity3D" });
```

- `size: number` → cubic `[n, n, n]`
- `size: [width, height, depth]` → non-cubic volume

## `Texture3DOptions`

| Option         | Default           | Description                                    |
| -------------- | ----------------- | ---------------------------------------------- |
| `label`        | `"Texture3D"`     | GPU debug labels                               |
| `format`       | `"rgba32float"`   | Texture format (must support storage + sample) |
| `usage`        | see below         | Override `TEXTURE_BINDING \| STORAGE_BINDING`  |
| `addressModeU` | `"mirror-repeat"` | Sampler wrap U                                 |
| `addressModeV` | `"mirror-repeat"` | Sampler wrap V                                 |
| `addressModeW` | `"mirror-repeat"` | Sampler wrap W                                 |
| `magFilter`    | `"linear"`        | Magnification filter                           |
| `minFilter`    | `"linear"`        | Minification filter                            |

Default usage: `GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.STORAGE_BINDING`.

## Instance

| Member        | Type               | Description                                              |
| ------------- | ------------------ | -------------------------------------------------------- |
| `width`       | `number`           | Voxel width (X)                                          |
| `height`      | `number`           | Voxel height (Y)                                         |
| `depth`       | `number`           | Voxel depth (Z)                                          |
| `format`      | `GPUTextureFormat` | Stored format                                            |
| `view`        | `GPUTextureView`   | `texture_3d` sampling binding                            |
| `storageView` | `GPUTextureView`   | `texture_storage_3d<rgba32float, write>` compute binding |
| `sampler`     | `GPUSampler`       | For `textureSample` in render shaders                    |
| `gpu`         | `GPUTexture`       | Escape hatch for advanced consumers                      |

### `destroy()`

Destroys the `GPUTexture`. Samplers do not require explicit destruction.

## Compute bind group usage

```ts
const volume = Texture3D.create(device, 32, { label: "Velocity3D" });

const computeBindGroup = BindGroup.create(device, compute.getBindGroupLayout(0), [
  { binding: 0, resource: uniformBuffer },
  { binding: 1, resource: volume.storageView },
]);
```

WGSL:

```wgsl
@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var velocityOut: texture_storage_3d<rgba32float, write>;
```

## Draw bind group usage

Use with `createSceneTexture3DPipelineLayout` and `BindGroup.create`:

```ts
const { pipelineLayout, bindGroupLayout } = createSceneTexture3DPipelineLayout(device);

const bindGroup = BindGroup.create(device, bindGroupLayout, [
  { binding: 0, resource: uniformBuffer },
  { binding: 1, resource: volume.view },
  { binding: 2, resource: volume.sampler },
]);
```

WGSL (`rgba32float` is unfilterable — use `textureLoad`, not `textureSample`):

```wgsl
@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var velocityTex: texture_3d<f32>;

let coord = vec3<i32>(uvw * vec3<f32>(textureDimensions(velocityTex)));
let value = textureLoad(velocityTex, coord, 0);
```

`AxisHelper` can reuse the same bind group (it only samples binding 0).

## Ping-pong helper

For compute passes that read the previous frame and write the next, use `Texture3DPingPong`:

```ts
import { Texture3DPingPong } from "belfast";

const pingPong = Texture3DPingPong.create(device, 32, { label: "Velocity3D" });

// Bind read.view as input, write.storageView as output
pingPong.swap();
```

See [texture3d-curl-noise example](../../examples/texture3d-curl-noise/src/main.ts).
