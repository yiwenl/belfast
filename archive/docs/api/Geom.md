# Geom

Procedural primitive generators with indexed geometry output.

## Import

```ts
import {
  Geom,
  type GeometryData,
  type PlaneOptions,
  type SphereOptions,
  type CubeOptions,
} from "belfast";
```

## Output shape

Each generator returns:

```ts
{
  positions: Float32Array;
  uvs: Float32Array;
  normals: Float32Array;
  indices: Uint16Array | Uint32Array;
}
```

## Methods

### `Geom.plane(options?)`

Options:

- `width` (default `1`)
- `height` (default `1`)
- `segmentsX` (default `1`)
- `segmentsY` (default `1`)

### `Geom.sphere(options?)`

Options:

- `radius` (default `1`)
- `segments` (default `12`)

### `Geom.cube(options?)`

Options:

- `size` (default `1`)

## Using with `Mesh`

```ts
const geom = Geom.cube({ size: 1 });
const positionBuffer = Buffer.fromData(device, geom.positions, BufferUsage.vertex);
const normalBuffer = Buffer.fromData(device, geom.normals, BufferUsage.vertex);
const indexBuffer = Buffer.fromData(device, geom.indices, BufferUsage.index);

const mesh = new Mesh(geom.positions.length / 3)
  .addVertexBuffer({
    buffer: positionBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
  })
  .addVertexBuffer({
    buffer: normalBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
  })
  .setIndexBuffer(
    indexBuffer,
    geom.indices.length,
    geom.indices.BYTES_PER_ELEMENT === 2 ? "uint16" : "uint32",
  );
```

`Draw.draw(pass, mesh)` automatically uses `drawIndexed` when an index buffer is set on the mesh.
