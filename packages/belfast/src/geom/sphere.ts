/**
 * Unit-sphere positions expanded to a triangle list (no index buffer).
 * Ported from alfrid `Geom.sphere`.
 */
export function createSphereTriangleList(radius: number, numSegments: number): Float32Array {
  const positions: number[] = [];

  function getPosition(i: number, j: number, isNormal = false): [number, number, number] {
    const rx = (i / numSegments) * Math.PI - Math.PI * 0.5;
    const ry = (j / numSegments) * Math.PI * 2;
    const r = isNormal ? 1 : radius;
    let x = Math.cos(ry) * Math.cos(rx) * r;
    let y = Math.sin(rx) * r;
    let z = Math.sin(ry) * Math.cos(rx) * r;

    const precision = 10000;
    x = Math.round(x * precision) / precision;
    y = Math.round(y * precision) / precision;
    z = Math.round(z * precision) / precision;

    return [x, y, z];
  }

  for (let i = 0; i < numSegments; i++) {
    for (let j = 0; j < numSegments; j++) {
      const p0 = getPosition(i, j);
      const p1 = getPosition(i + 1, j);
      const p2 = getPosition(i + 1, j + 1);
      const p3 = getPosition(i, j + 1);

      const quad = [p0, p1, p2, p3];
      const tri0 = [quad[0], quad[1], quad[2]];
      const tri1 = [quad[0], quad[2], quad[3]];

      for (const tri of [tri0, tri1]) {
        for (const p of tri) {
          positions.push(p[0], p[1], p[2]);
        }
      }
    }
  }

  return new Float32Array(positions);
}
