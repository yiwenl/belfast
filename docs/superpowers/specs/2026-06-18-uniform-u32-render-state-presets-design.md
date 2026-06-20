# Uniform u32 And Render State Presets Design

## Goal

Add two low-risk Belfast ergonomics features: unsigned integer scalar fields in `UniformBlock`, and reusable render-state presets for common opaque and depth-only triangle draws. Defer a paired `ShadowCaster` helper until more callers prove the abstraction.

## Scope

- Add `u32` to `UniformFieldType`.
- Preserve existing float, vector, and matrix uniform packing behavior.
- Add pure render-state preset helpers that return option fragments for existing `Draw` and `DepthDraw` constructors.
- Document the new APIs.
- Add regression tests that can run without new dependencies.

## Non-Goals

- Do not add `ShadowCaster` yet.
- Do not introduce a scene graph, renderable object hierarchy, or wrapper that owns bind groups.
- Do not replace existing `Draw` or `DepthDraw` constructor options.

## Architecture

`UniformBlock` will keep a single `ArrayBuffer` with both `Float32Array` and `Uint32Array` views. Float fields continue to write through the float view; `u32` fields write through the unsigned integer view at the same 4-byte-aligned offset. `toFloat32Array()` and `data` remain available for existing callers, while `writeToBuffer()` still uploads the same packed bytes.

Render-state presets will live in a small helper module and return plain object fragments. Callers can spread the fragments into `new Draw(...)` or `new DepthDraw(...)`, then override any field after the spread when needed.

## API

```ts
const uniforms = UniformBlock.create({
  time: "f32",
  count: "u32",
});

uniforms.set("time", 1.5).set("count", 123);
```

```ts
new Draw(device, shaderCode, {
  label: "OpaqueMesh",
  layout,
  vertexBuffers,
  ...opaqueTriangles({
    colorFormat: device.format,
    depthFormat: "depth24plus",
    cullMode: "back",
  }),
});

new DepthDraw(device, shadowShaderCode, {
  label: "ShadowMesh",
  layout: shadowLayout,
  vertexBuffers,
  ...depthOnlyTriangles({
    depthFormat: "depth32float",
    cullMode: "back",
  }),
});
```

## Validation

`u32` fields accept only finite non-negative integers in the `u32` range. Fractional, negative, non-finite, and array-like values throw explicit errors.

## Testing

Use Node's built-in `node:test` against the built package. Tests cover mixed `f32`/`u32` packing, `u32` validation, and preset output matching equivalent explicit pipeline state.
