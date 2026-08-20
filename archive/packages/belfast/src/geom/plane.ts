export type PlaneAxis = "xy" | "xz" | "yz";

export interface PlaneTriangleList {
  positions: Float32Array;
  uvs: Float32Array;
}

/**
 * Centered plane expanded to a triangle list (no index buffer).
 * Ported from alfrid `Geom.plane`.
 */
export function createPlaneTriangleList(
  width: number,
  height: number,
  segments = 1,
  axis: PlaneAxis = "xy",
): PlaneTriangleList {
  const positions: number[] = [];
  const uvs: number[] = [];
  const gapX = width / segments;
  const gapY = height / segments;
  const gapUV = 1 / segments;
  const sx = -width * 0.5;
  const sy = -height * 0.5;

  function pushTriangle(
    a: [number, number, number],
    b: [number, number, number],
    c: [number, number, number],
    ua: [number, number],
    ub: [number, number],
    uc: [number, number],
  ): void {
    for (const p of [a, b, c]) {
      positions.push(p[0], p[1], p[2]);
    }
    for (const uv of [ua, ub, uc]) {
      uvs.push(uv[0], uv[1]);
    }
  }

  for (let i = 0; i < segments; i++) {
    for (let j = 0; j < segments; j++) {
      const tx = gapX * i + sx;
      const ty = gapY * j + sy;
      const u = i / segments;
      const v = j / segments;

      let p0: [number, number, number];
      let p1: [number, number, number];
      let p2: [number, number, number];
      let p3: [number, number, number];
      let uv0: [number, number];
      let uv1: [number, number];
      let uv2: [number, number];
      let uv3: [number, number];

      if (axis === "xz") {
        p0 = [tx, 0, ty + gapY];
        p1 = [tx + gapX, 0, ty + gapY];
        p2 = [tx + gapX, 0, ty];
        p3 = [tx, 0, ty];
        uv0 = [u, 1 - (v + gapUV)];
        uv1 = [u + gapUV, 1 - (v + gapUV)];
        uv2 = [u + gapUV, 1 - v];
        uv3 = [u, 1 - v];
      } else if (axis === "yz") {
        p0 = [0, ty, tx];
        p1 = [0, ty, tx + gapX];
        p2 = [0, ty + gapY, tx + gapX];
        p3 = [0, ty + gapY, tx];
        uv0 = [u, v];
        uv1 = [u + gapUV, v];
        uv2 = [u + gapUV, v + gapUV];
        uv3 = [u, v + gapUV];
      } else {
        p0 = [tx, ty, 0];
        p1 = [tx + gapX, ty, 0];
        p2 = [tx + gapX, ty + gapY, 0];
        p3 = [tx, ty + gapY, 0];
        uv0 = [u, v];
        uv1 = [u + gapUV, v];
        uv2 = [u + gapUV, v + gapUV];
        uv3 = [u, v + gapUV];
      }

      pushTriangle(p0, p1, p2, uv0, uv1, uv2);
      pushTriangle(p0, p2, p3, uv0, uv2, uv3);
    }
  }

  return {
    positions: new Float32Array(positions),
    uvs: new Float32Array(uvs),
  };
}

export interface BillboardDiscTriangle {
  /**
   * 3 vertices × 4 floats: xy = billboard offset (unit disc radius 0.5),
   * zw = disc UV (circle centered at 0.5, 0.5 — use `discard` outside radius 0.5).
   */
  corners: Float32Array;
  vertexCount: number;
}

/** Single-triangle billboard that circumscribes a unit disc (replaces a 2-triangle quad). */
export function createBillboardDiscTriangle(): BillboardDiscTriangle {
  const r = 1 / Math.sqrt(3);
  return {
    corners: new Float32Array([0, r, 0.5, 1, -0.5, -r * 0.5, 0, 0, 0.5, -r * 0.5, 1, 0]),
    vertexCount: 3,
  };
}
