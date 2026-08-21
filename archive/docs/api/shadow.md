# Shadow WGSL Utilities

Reusable WGSL code snippets for handling shadows efficiently without running into WebGPU uniform control flow validation errors.

## Import

```ts
import { wgslShadowPcf3x3 } from "belfast";
```

## `wgslShadowPcf3x3`

A string containing a WGSL function that implements Percentage-Closer Filtering (PCF) with a 3x3 footprint.
It utilizes `textureSampleCompareLevel` under the hood to bypass branching requirements, making it safe to use in a fragment shader after a `discard` statement or non-uniform control flow.

### WGSL Signature

```wgsl
fn sampleShadowPcf3x3(
  shadowMap: texture_depth_2d,
  shadowSampler: sampler_comparison,
  shadowCoord: vec4<f32>,
  mapSize: f32,
  bias: f32,
) -> f32
```

### Usage

```ts
const drawShader =
  wgslShadowPcf3x3 +
  `
// ... rest of shader
let shadow = sampleShadowPcf3x3(shadowMap, shadowSampler, input.shadowCoord, 1024.0, 0.001);
`;
```
