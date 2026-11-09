import { seededRandom } from "./hash";
import { GRID, WORLD_SIZE } from "./terrain";

type Point = [number, number];
type Site = { point: Point; angle: number };
const dot = (a: Point, b: Point) => a[0] * b[0] + a[1] * b[1];

function cellPolygon(site: Site, sites: Site[]): Point[] {
  let polygon: Point[] = [
    [0, 0],
    [WORLD_SIZE, 0],
    [WORLD_SIZE, WORLD_SIZE],
    [0, WORLD_SIZE],
  ];
  for (const other of sites) {
    if (other === site) continue;
    const normal: Point = [other.point[0] - site.point[0], other.point[1] - site.point[1]];
    const limit = (dot(other.point, other.point) - dot(site.point, site.point)) * 0.5;
    const clipped: Point[] = [];
    for (let i = 0; i < polygon.length; i++) {
      const a = polygon[i],
        b = polygon[(i + 1) % polygon.length];
      const da = dot(a, normal) - limit,
        db = dot(b, normal) - limit;
      if (da <= 0) clipped.push(a);
      if ((da < 0 && db > 0) || (da > 0 && db < 0)) {
        const t = da / (da - db);
        clipped.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
      }
    }
    polygon = clipped;
  }
  return polygon;
}

// Only the XZ layout is built here; GPU compute adds height, shading and visibility each frame.
export function generateVoronoiSegments(seed: string, count: number, spacing: number) {
  const random = seededRandom(`${seed}:voronoi`);
  const sites: Site[] = Array.from({ length: count }, () => ({
    point: [random() * WORLD_SIZE, random() * WORLD_SIZE],
    angle: random() * Math.PI,
  }));
  const data: number[] = [];
  const scale = (GRID - 1) / WORLD_SIZE;
  const order = [0, 4, 2, 6, 1, 5, 3, 7];
  let stroke = 0;
  for (const site of sites) {
    const polygon = cellPolygon(site, sites);
    if (polygon.length < 3) continue;
    const edges = polygon.flatMap((a, i) => {
      const b = polygon[(i + 1) % polygon.length];
      const length = Math.hypot(b[0] - a[0], b[1] - a[1]);
      if (length < 1e-12) return [];
      const nx = (a[1] - b[1]) / length,
        nz = (b[0] - a[0]) / length;
      return [[nx, nz, (nx * a[0] + nz * a[1]) * scale]];
    });
    const clearance = (p: Point) => {
      let distance = Infinity;
      for (const [nx, nz, offset] of edges)
        distance = Math.min(distance, nx * p[0] + nz * p[1] - offset);
      return Math.max(0, distance);
    };
    const direction: Point = [Math.cos(site.angle), Math.sin(site.angle)];
    const normal: Point = [-direction[1], direction[0]];
    const origin = dot(site.point, normal);
    const offsets = polygon.map((p) => dot(p, normal));
    const first = Math.ceil((Math.min(...offsets) - origin) / spacing);
    const last = Math.floor((Math.max(...offsets) - origin) / spacing);
    for (let line = first; line <= last; line++) {
      const offset = origin + line * spacing;
      let low = Infinity,
        high = -Infinity;
      for (let edge = 0; edge < polygon.length; edge++) {
        const a = polygon[edge],
          b = polygon[(edge + 1) % polygon.length];
        const da = dot(a, normal) - offset,
          db = dot(b, normal) - offset;
        if (Math.abs(da - db) < 1e-12) continue;
        const t = da / (da - db);
        if (t < 0 || t > 1) continue;
        const along = dot([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t], direction);
        low = Math.min(low, along);
        high = Math.max(high, along);
      }
      if (high - low < 1e-6 || !Number.isFinite(low + high)) continue;
      const steps = Math.ceil((high - low) * scale);
      const rank = (order[((line % 8) + 8) % 8] + 0.5) / 8;
      const point = (t: number): Point => [
        Math.max(0, Math.min(GRID - 1, (normal[0] * offset + direction[0] * t) * scale)),
        Math.max(0, Math.min(GRID - 1, (normal[1] * offset + direction[1] * t) * scale)),
      ];
      let a = point(low);
      let ca = clearance(a);
      for (let step = 1; step <= steps; step++) {
        const b = point(low + ((high - low) * step) / steps);
        const cb = clearance(b);
        data.push(...a, ...b, rank, stroke, ca, cb);
        a = b;
        ca = cb;
      }
      stroke++;
    }
  }
  return Float32Array.from(data);
}
