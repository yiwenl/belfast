# Feature: Vertex buffers

## Summary

Vertex positions move from hardcoded WGSL (`@builtin(vertex_index)`) into GPU vertex buffers. `Buffer` is a shareable resource; `Mesh` groups one or more vertex buffer bindings; `Draw` consumes mesh layouts at pipeline creation and binds buffers each frame.

## Public API

| Symbol        | Role                                                           |
| ------------- | -------------------------------------------------------------- |
| `Buffer`      | Wraps `GPUBuffer` with create / fromData / write / destroy     |
| `BufferUsage` | Presets: `vertex`, `storage`, `vertexStorage`                  |
| `Mesh`        | Vertex count + `addVertexBuffer` + `bind` + `getVertexLayouts` |
| `Draw`        | `vertexBuffers` in options; `draw(pass, mesh)`                 |

## Triangle usage

```ts
const positionBuffer = Buffer.fromData(device, positions, BufferUsage.vertex, "triangle-positions");

const mesh = new Mesh(3).addVertexBuffer({
  buffer: positionBuffer,
  arrayStride: 8,
  attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
  slot: 0,
});

const draw = new Draw(device, shaderCode, {
  label: "Triangle",
  vertexBuffers: mesh.getVertexLayouts(),
});

draw.draw(pass, mesh);
```

WGSL uses `@location(0) position: vec2<f32>` instead of a positions array in the shader.

## Adding another vertex buffer

```ts
mesh
  .addVertexBuffer({ buffer: positionBuffer, /* slot 0 */ ... })
  .addVertexBuffer({
    buffer: colorBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
    slot: 1,
  });
```

Update WGSL `VertexInput` and recreate `Draw` with `vertexBuffers: mesh.getVertexLayouts()`.

## Sharing buffers between passes

Create with `BufferUsage.vertexStorage` so the same `GPUBuffer` can be written in a compute pass (storage binding) and read in a render pass (vertex slot):

```ts
const buffer = Buffer.create(device, byteSize, BufferUsage.vertexStorage, "shared-positions");
// compute pass writes positions ...
// render pass:
mesh.addVertexBuffer({ buffer, arrayStride: 12, attributes: [...] });
```

No extra copy if usages are declared at creation time.

## Feedback (this feature)

_Use this section for review notes specific to vertex buffers._

-

## Out of scope

- Index buffers / `drawIndexed`
- Bind groups and uniforms
- Instancing
