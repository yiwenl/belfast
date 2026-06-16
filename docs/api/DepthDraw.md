# DepthDraw

A wrapper around [Draw](Draw.md) optimized for depth-only rendering (e.g. shadow map generation passes).
Automatically configures the color targets to `[]` and sets up a default depth stencil if omitted.

## Import

```ts
import { DepthDraw, type DepthDrawOptions } from "belfast";
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
