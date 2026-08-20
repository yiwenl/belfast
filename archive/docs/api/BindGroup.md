# BindGroup

Wraps a `GPUBindGroup` for binding resources (uniform buffers, textures, samplers) to a render pass.

## Import

```ts
import { BindGroup, type BindGroupResource } from "belfast";
```

## Types

### `BindGroupResource`

| Field      | Type                           | Description                       |
| ---------- | ------------------------------ | --------------------------------- |
| `binding`  | `number`                       | WGSL `@binding` index             |
| `resource` | `GPUBindingResource \| Buffer` | WebGPU resource or Belfast buffer |

Pass a Belfast `Buffer` for uniform/storage buffer bindings; pass `GPUTextureView`, `GPUSampler`, etc. directly for other binding types.

## Static methods

### `BindGroup.create(device, layout, buffer, binding?, label?)`

Creates a bind group with a single uniform buffer entry. Convenience wrapper around the multi-resource overload.

| Argument  | Default | Description                                      |
| --------- | ------- | ------------------------------------------------ |
| `device`  | —       | Belfast `Device`                                 |
| `layout`  | —       | From `draw.getBindGroupLayout(0)` after pipeline |
| `buffer`  | —       | Uniform `Buffer` (`BufferUsage.uniform`)         |
| `binding` | `0`     | WGSL `@binding` index                            |
| `label`   | —       | Debug label                                      |

### `BindGroup.create(device, layout, resources, label?)`

Creates a bind group with multiple entries (uniform buffers, textures, samplers, etc.):

```ts
const bindGroup = BindGroup.create(
  device,
  layout,
  [
    { binding: 0, resource: uniformBuffer },
    { binding: 1, resource: textureView },
    { binding: 2, resource: sampler },
  ],
  "material-bind-group",
);
```

### `BindGroup.createFromEntries(device, layout, resources, label?)`

Same as the `resources` array overload; use when you prefer an explicit method name.

Create the bind group **once** after `Draw` constructs the pipeline. Update uniform buffers each frame with `buffer.write()`; reuse the same bind group.

## Instance

| Property | Type           |
| -------- | -------------- |
| `gpu`    | `GPUBindGroup` |

### `bind(passEncoder, groupIndex?)`

Calls `setBindGroup(groupIndex, this.gpu)`. Default `groupIndex` is `0`.

Usually passed to `Draw.draw(pass, mesh, bindGroup)` instead of calling `bind` directly.

### Lifecycle

`GPUBindGroup` does not need an explicit `destroy()` — it is released when no longer referenced (unlike `GPUBuffer`, which uses `Buffer.destroy()`). Destroy any `Buffer` instances you bound when tearing down resources.

## WGSL requirements

Declare uniforms at `@group(0) @binding(0)` (or match your `binding` values):

```wgsl
struct SceneUniforms {
  time: vec4<f32>,
}
@group(0) @binding(0) var<uniform> scene: SceneUniforms;
```

Use `Buffer.uniformSize()` when allocating uniform buffers. Buffer size must be at least the shader’s `minBindingSize` (WGSL struct layout can be larger than the sum of fields — e.g. `f32` + `vec3` pads to 32 bytes; a single `vec4` is 16).

## Uniform binding offsets

`Buffer.uniformSize()` aligns **struct sizes** to 16 bytes. If you use **dynamic offsets** in `setBindGroup`, or pack multiple structs into one uniform buffer, each bind offset must align to the device’s `minUniformBufferOffsetAlignment` (typically **256 bytes**). Query `device.device.limits.minUniformBufferOffsetAlignment`.

## See also

- [Buffer](Buffer.md) — `BufferUsage.uniform`, `uniformSize`
- [Draw](Draw.md) — `getBindGroupLayout`, `draw(..., bindGroup)`
- [Example: triangle-time](../../examples/triangle-time/src/main.ts)
