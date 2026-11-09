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
export function generateVoronoiSegments(
  seed: string,
  count: number,
  spacing: number,
  curve: number,
) {
  const random = seededRandom(`${seed}:voronoi`);
  const sites: Site[] = Array.from({ length: count }, () => ({
    point: [random() * WORLD_SIZE, random() * WORLD_SIZE],
    angle: random() * Math.PI,
  }));
  const data: number[] = [];
  const scale = (GRID - 1) / WORLD_SIZE;
  const inGrid = (v: number) => Math.max(0, Math.min(GRID - 1, v));
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
    // Signed distance to the cell border in grid units; negative outside.
    const clearance = (p: Point) => {
      let distance = Infinity;
      for (const [nx, nz, offset] of edges)
        distance = Math.min(distance, nx * p[0] + nz * p[1] - offset);
      return distance;
    };
    const direction: Point = [Math.cos(site.angle), Math.sin(site.angle)];
    const normal: Point = [-direction[1], direction[0]];
    const origin = dot(site.point, normal);
    const center = dot(site.point, direction);
    // Each cell's lines are copies of one parabola shifted along the normal, so spacing holds.
    const bend = curve * (random() * 2 - 1);
    const offsets = polygon.map((p) => dot(p, normal));
    const alongs = polygon.map((p) => dot(p, direction));
    const low = Math.min(...alongs),
      high = Math.max(...alongs);
    const sag = Math.abs(bend) * 0.5 * Math.max((low - center) ** 2, (high - center) ** 2);
    const first = Math.ceil((Math.min(...offsets) - sag - origin) / spacing);
    const last = Math.floor((Math.max(...offsets) + sag - origin) / spacing);
    const steps = Math.ceil((high - low) * scale);
    for (let line = first; line <= last; line++) {
      const offset = origin + line * spacing;
      const rank = (order[((line % 8) + 8) % 8] + 0.5) / 8;
      const point = (t: number): Point => {
        const n = offset + 0.5 * bend * (t - center) ** 2;
        return [
          (normal[0] * n + direction[0] * t) * scale,
          (normal[1] * n + direction[1] * t) * scale,
        ];
      };
      const push = (a: Point, b: Point, ca: number, cb: number) =>
        data.push(...a.map(inGrid), ...b.map(inGrid), rank, stroke, ca, cb, ...direction, 0, 0);
      let run: Point | undefined;
      let runClearance = 0;
      let previousT = low;
      let previous = clearance(point(low));
      if (previous >= 0) {
        run = point(low);
        runClearance = previous;
      }
      for (let step = 1; step <= steps; step++) {
        const t = low + ((high - low) * step) / steps;
        const p = point(t);
        const d = clearance(p);
        if (previous >= 0 !== d >= 0) {
          // The border is straight, so interpolating the signed distance finds the crossing.
          const cross = point(previousT + ((t - previousT) * previous) / (previous - d));
          if (run) {
            push(run, cross, runClearance, 0);
            run = undefined;
            stroke++;
          } else {
            run = cross;
            runClearance = 0;
          }
        }
        if (run && d >= 0) {
          push(run, p, runClearance, d);
          run = p;
          runClearance = d;
        }
        previousT = t;
        previous = d;
      }
      if (run) stroke++;
    }
  }
  return Float32Array.from(data);
}
