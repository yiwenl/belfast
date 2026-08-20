export interface GeometryData {
  positions: Float32Array;
  uvs: Float32Array;
  normals: Float32Array;
  indices: Uint16Array | Uint32Array;
}

function toIndexArray(indices: number[], vertexCount: number): Uint16Array | Uint32Array {
  return vertexCount > 65535 ? new Uint32Array(indices) : new Uint16Array(indices);
}

export interface PlaneOptions {
  width?: number;
  height?: number;
  segmentsX?: number;
  segmentsY?: number;
}

export interface SphereOptions {
  radius?: number;
  segments?: number;
}

export interface CubeOptions {
  size?: number;
}

export class Geom {
  static plane(options: PlaneOptions = {}): GeometryData {
    const width = options.width ?? 1;
    const height = options.height ?? 1;
    const segmentsX = Math.max(1, Math.floor(options.segmentsX ?? 1));
    const segmentsY = Math.max(1, Math.floor(options.segmentsY ?? 1));

    const positions: number[] = [];
    const uvs: number[] = [];
    const normals: number[] = [];
    const indices: number[] = [];

    for (let y = 0; y <= segmentsY; y++) {
      for (let x = 0; x <= segmentsX; x++) {
        const u = x / segmentsX;
        const v = y / segmentsY;
        const px = (u - 0.5) * width;
        const py = (v - 0.5) * height;
        positions.push(px, py, 0);
        uvs.push(u, v);
        normals.push(0, 0, 1);
      }
    }

    const row = segmentsX + 1;
    for (let y = 0; y < segmentsY; y++) {
      for (let x = 0; x < segmentsX; x++) {
        const a = y * row + x;
        const b = a + 1;
        const c = a + row + 1;
        const d = a + row;
        indices.push(a, b, c, a, c, d);
      }
    }

    return {
      positions: new Float32Array(positions),
      uvs: new Float32Array(uvs),
      normals: new Float32Array(normals),
      indices: toIndexArray(indices, positions.length / 3),
    };
  }

  static sphere(options: SphereOptions = {}): GeometryData {
    const radius = options.radius ?? 1;
    const segments = Math.max(3, Math.floor(options.segments ?? 12));

    const positions: number[] = [];
    const uvs: number[] = [];
    const normals: number[] = [];
    const indices: number[] = [];

    for (let y = 0; y <= segments; y++) {
      const v = y / segments;
      const phi = v * Math.PI;
      const cosPhi = Math.cos(phi);
      const sinPhi = Math.sin(phi);
      for (let x = 0; x <= segments; x++) {
        const u = x / segments;
        const theta = u * Math.PI * 2;
        const cosTheta = Math.cos(theta);
        const sinTheta = Math.sin(theta);

        const nx = cosTheta * sinPhi;
        const ny = cosPhi;
        const nz = sinTheta * sinPhi;

        positions.push(nx * radius, ny * radius, nz * radius);
        normals.push(nx, ny, nz);
        uvs.push(u, 1 - v);
      }
    }

    const row = segments + 1;
    for (let y = 0; y < segments; y++) {
      for (let x = 0; x < segments; x++) {
        const a = y * row + x;
        const b = a + 1;
        const c = a + row + 1;
        const d = a + row;
        indices.push(a, b, c, a, c, d);
      }
    }

    return {
      positions: new Float32Array(positions),
      uvs: new Float32Array(uvs),
      normals: new Float32Array(normals),
      indices: toIndexArray(indices, positions.length / 3),
    };
  }

  static cube(options: CubeOptions = {}): GeometryData {
    const size = options.size ?? 1;
    const h = size * 0.5;

    const positions: number[] = [];
    const uvs: number[] = [];
    const normals: number[] = [];
    const indices: number[] = [];
    let base = 0;

    function pushFace(
      p0: [number, number, number],
      p1: [number, number, number],
      p2: [number, number, number],
      p3: [number, number, number],
      n: [number, number, number],
    ): void {
      positions.push(...p0, ...p1, ...p2, ...p3);
      uvs.push(0, 0, 1, 0, 1, 1, 0, 1);
      normals.push(...n, ...n, ...n, ...n);
      indices.push(base + 0, base + 1, base + 2, base + 0, base + 2, base + 3);
      base += 4;
    }

    pushFace([h, -h, -h], [h, h, -h], [h, h, h], [h, -h, h], [1, 0, 0]);
    pushFace([-h, -h, h], [-h, h, h], [-h, h, -h], [-h, -h, -h], [-1, 0, 0]);
    pushFace([-h, h, -h], [-h, h, h], [h, h, h], [h, h, -h], [0, 1, 0]);
    pushFace([-h, -h, h], [-h, -h, -h], [h, -h, -h], [h, -h, h], [0, -1, 0]);
    pushFace([-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h], [0, 0, 1]);
    pushFace([h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h], [0, 0, -1]);

    return {
      positions: new Float32Array(positions),
      uvs: new Float32Array(uvs),
      normals: new Float32Array(normals),
      indices: toIndexArray(indices, positions.length / 3),
    };
  }
}
