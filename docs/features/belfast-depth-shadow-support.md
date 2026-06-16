# Feature Request: Depth Texture and Shadow Map Support for Belfast

## Summary

Add first-class depth texture, depth-only render pass, shadow map, and camera fitting utilities to Belfast. The `particles` experiment needed these features to render particle self-shadowing from an orthographic light camera, but several pieces had to be assembled manually outside the library.

## Background

The `experiments2/apps/particles` app uses Belfast to:

- initialize and render 200,000 GPU particles
- update particles with a compute shader
- render camera-facing particle billboards
- render a shadow map from a top-right light source
- sample that shadow map in the main particle shader

The experiment works, but shadow support exposed gaps in the current Belfast API.

## Problems Observed

### Depth Textures Cannot Be Sampled

`RenderTarget` can create a depth attachment, but its depth texture is created only with `GPUTextureUsage.RENDER_ATTACHMENT`. For shadow mapping, the depth texture also needs `GPUTextureUsage.TEXTURE_BINDING`.

The experiment had to create the shadow depth texture manually:

```ts
device.gpu.createTexture({
  size: [1024, 1024],
  format: "depth32float",
  usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
});
```

### No First-class Depth-only Pass

The shadow map pass uses a draw pipeline with no color targets. This is valid WebGPU, but Belfast does not make this pattern explicit. The user has to know to pass `targets: []` and configure a compatible depth stencil state.

### Shadow Sampling Has WGSL Gotchas

WGSL requires `textureSampleCompare` to be called only from uniform control flow. In particle rendering, shadow coordinates vary per fragment, so the shader needed `textureSampleCompareLevel` and branchless out-of-bounds handling.

This is subtle enough that Belfast should provide a reusable shadow sampling helper or example.

### Light Camera Bounds Are Easy to Get Wrong

The first orthographic light camera bounds were too tight. The particle simulation allowed positions to overshoot `MAX_RADIUS` before being pulled back, so the shadow camera had to fit the actual possible particle bounds, not just the nominal radius.

## Proposed API Additions

### 1. Sampleable Depth Texture Support

Allow `RenderTarget` depth textures to be sampled:

```ts
const shadowTarget = RenderTarget.create(device, {
  width: 1024,
  height: 1024,
  withDepth: true,
  depthFormat: "depth32float",
  depthTextureUsage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
});
```

Expose:

```ts
shadowTarget.depthView;
shadowTarget.depthTexture;
```

### 2. Shadow Map Helper

Provide a small wrapper for common shadow map setup:

```ts
const shadowMap = ShadowMap.create(device, {
  size: 1024,
  format: "depth32float",
  filter: "linear",
});

const pass = shadowMap.beginRenderPass(encoder);
```

Expected fields:

```ts
shadowMap.texture;
shadowMap.view;
shadowMap.sampler; // comparison sampler
shadowMap.size;
shadowMap.destroy();
```

### 3. Depth-only Draw Support

Make depth-only rendering explicit:

```ts
const shadowDraw = new DepthDraw(device, shadowShaderCode, {
  layout,
  vertexBuffers,
  depthFormat: "depth32float",
  depthCompare: "less",
});
```

Alternative: keep `Draw`, but document and type-test:

```ts
new Draw(device, shaderCode, {
  targets: [],
  depthStencil: {
    format: "depth32float",
    depthWriteEnabled: true,
    depthCompare: "less",
  },
});
```

### 4. Orthographic Camera Fitting

Add camera fitting helpers that compute the minimum useful orthographic bounds:

```ts
fitOrthographicCameraToSphere({
  camera,
  center: [0, 0, 0],
  radius,
  eye: lightPosition,
  up: [0, 0, -1],
  padding: 0.75,
});
```

For more general usage:

```ts
fitOrthographicCameraToBounds({
  camera,
  points,
  eye: lightPosition,
  target: [0, 0, 0],
  up,
  padding,
});
```

The helper should return or apply:

```ts
{
  left,
  right,
  bottom,
  top,
  near,
  far,
}
```

### 5. WGSL Shadow Sampling Helper

Provide a reusable WGSL snippet for shadow lookup:

```wgsl
fn sampleShadowPcf3x3(
  shadowMap: texture_depth_2d,
  shadowSampler: sampler_comparison,
  shadowCoord: vec4<f32>,
  mapSize: f32,
  bias: f32,
) -> f32
```

Implementation should use `textureSampleCompareLevel` and avoid non-uniform control flow around texture sampling.

## Acceptance Criteria

- `RenderTarget` can create a depth texture that is both renderable and sampleable.
- Belfast exposes a comparison sampler path with linear filtering for shadow maps.
- A depth-only render pass can be created without manual low-level WebGPU setup.
- An orthographic light camera can be fitted to a sphere or point bounds with padding.
- Belfast includes a shadow map example or test based on a depth pass plus shadow lookup.
- WGSL shadow helper avoids the `textureSampleCompare must only be called from uniform control flow` validation error.

## Priority

High:

- sampleable depth texture support
- comparison sampler support
- depth-only render pass ergonomics

Medium:

- orthographic camera fitting helpers
- WGSL shadow snippets
- depth texture debug preview

## Notes From Particles Experiment

- Particle count: 200,000
- Shadow map size: 1024
- Shadow format: `depth32float`
- Light position used: `[2, 18, 0.5]`
- Particle simulation can overshoot the nominal radius, so fitting should use actual bounds rather than only the attractor radius.
