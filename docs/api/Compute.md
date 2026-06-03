# Compute

Builds a compute pipeline from WGSL source and dispatches it on a compute pass encoder. The compute-shader counterpart of [`Draw`](Draw.md).

## Import

```ts
import { Compute, type ComputeOptions, type WorkgroupCount } from "belfast";
import type { BindGroup } from "belfast";
```

## Constructor

```ts
new Compute(device: Device, shaderCode: string, optionsOrLabel?: ComputeOptions | string)
```

| Argument         | Description                                                               |
| ---------------- | ------------------------------------------------------------------------- |
| `device`         | Belfast `Device` (uses `device.gpu`)                                      |
| `shaderCode`     | Full WGSL source string (e.g. from `import shader from "./foo.wgsl?raw"`) |
| `optionsOrLabel` | Optional `ComputeOptions` object or label string                          |

### `ComputeOptions`

| Field        | Type                          | Default     | Description                                                                |
| ------------ | ----------------------------- | ----------- | -------------------------------------------------------------------------- |
| `label`      | `string`                      | `"Compute"` | Debug label prefix                                                         |
| `layout`     | `GPUPipelineLayout \| "auto"` | `"auto"`    | Pipeline layout; use a shared layout to reuse bind groups across pipelines |
| `entryPoint` | `string`                      | `"cs_main"` | Compute entry point                                                        |

Creates a `GPUShaderModule` and a `GPUComputePipeline`.

## Types

### `WorkgroupCount`

```ts
type WorkgroupCount = number | readonly [number, number] | readonly [number, number, number];
```

Number of workgroups to dispatch: `x`, `[x, y]`, or `[x, y, z]`.

## Methods

### `getBindGroupLayout(index?)`

Returns `pipeline.getBindGroupLayout(index)` for creating bind groups after the pipeline exists (default `index` is `0`). Use with `BindGroup.create(...)` when the pipeline uses `layout: "auto"`.

### `dispatch(passEncoder, bindGroup?, workgroups?)`

| Argument      | Default | Description                                           |
| ------------- | ------- | ----------------------------------------------------- |
| `passEncoder` | —       | Active `GPUComputePassEncoder`                        |
| `bindGroup`   | —       | Optional `BindGroup` or array (bound at indices 0..n) |
| `workgroups`  | `1`     | `WorkgroupCount` to dispatch                          |

Sets the pipeline and bind groups, then dispatches. Use this when batching multiple dispatches into one pass.

### `run(encoder, bindGroup?, workgroups?, label?)`

Convenience: begins a compute pass, calls `dispatch(...)`, and ends the pass — collapsing the begin/dispatch/end boilerplate into a single call.

| Argument     | Default | Description                                           |
| ------------ | ------- | ----------------------------------------------------- |
| `encoder`    | —       | Active `GPUCommandEncoder`                            |
| `bindGroup`  | —       | Optional `BindGroup` or array (bound at indices 0..n) |
| `workgroups` | `1`     | `WorkgroupCount` to dispatch                          |
| `label`      | —       | Optional compute pass label                           |

## WGSL requirements

Your shader module must define a compute entry point matching `entryPoint` (default `cs_main`):

```wgsl
@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) { ... }
```

## Example

```ts
const compute = new Compute(device, computeShaderCode, { label: "ParticlesCompute" });

const bindGroup = BindGroup.create(device, compute.getBindGroupLayout(0), [
  { binding: 0, resource: simUniformBuffer },
  { binding: 1, resource: particleBuffer },
]);

// Per frame, inside a command encoder:
compute.run(encoder, bindGroup, Math.ceil(PARTICLE_COUNT / 256), "particles-sim");
```

Equivalent explicit form (when sharing a pass with other dispatches):

```ts
const pass = encoder.beginComputePass({ label: "sim" });
compute.dispatch(pass, bindGroup, Math.ceil(PARTICLE_COUNT / 256));
pass.end();
```

## See also

- [Draw](Draw.md) — render-pipeline counterpart
- [BindGroup](BindGroup.md) — storage/uniform bind groups
- [Buffer](Buffer.md) — `BufferUsage.vertexStorage` for compute-written, instance-read buffers
- [Device](Device.md) — must be created first
