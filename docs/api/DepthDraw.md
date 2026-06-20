# DepthDraw

A wrapper around [Draw](Draw.md) optimized for depth-only rendering (e.g. shadow map generation passes).
Automatically configures the color targets to `[]` and sets up a default depth stencil if omitted.

## Import

```ts
import { DepthDraw, depthOnlyTriangles, type DepthDrawOptions } from "belfast";
```

## Constructor

```ts
new DepthDraw(device: Device, shaderCode: string, optionsOrLabel?: DepthDrawOptions | string)
```

### `DepthDrawOptions`

Extends `DrawOptions` excluding `targets`.

| Field               | Default          | Description                 |
| ------------------- | ---------------- | --------------------------- |
| `depthFormat`       | `"depth32float"` | Defaults to 32-bit float    |
| `depthCompare`      | `"less"`         | Default depth test function |
| `depthWriteEnabled` | `true`           | Default write state         |

## Methods

### `getBindGroupLayout(index = 0)`

Returns pipeline layout.

### `draw(...)`

Same arguments as `Draw.draw(...)`.

## Render-state presets

Use `depthOnlyTriangles(...)` to share common depth-only triangle-list state:

```ts
const shadowDraw = new DepthDraw(device, shadowShaderCode, {
  label: "ShadowMesh",
  layout: shadowPipelineLayout,
  vertexBuffers: mesh.getVertexLayouts(),
  ...depthOnlyTriangles({
    depthFormat: "depth32float",
    cullMode: "back",
  }),
});
```

The helper returns:

- `primitive: { topology: "triangle-list", cullMode }`
- `depthFormat`
- `depthWriteEnabled`
- `depthCompare`

Defaults are `cullMode: "back"`, `depthFormat: "depth32float"`, `depthCompare: "less"`, and `depthWriteEnabled: true`.
