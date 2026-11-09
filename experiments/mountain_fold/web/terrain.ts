import { seededRandom } from "./hash";

export const GRID = 512;
export const DROPLETS = 1024;
export const WORLD_SIZE = 12;
export interface TerrainSettings {
  seed: string;
  height: number;
  width: number;
  rounding: number;
  levels: number;
  direction: number;
  noise: number;
  noiseFloor: number;
  scale: number;
}
type Point = [number, number];

const smoothstep = (a: number, b: number, x: number) => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

export function generateTerrain(settings: TerrainSettings) {
  const random = seededRandom(settings.seed);
  const nodes: Point[] = [[0, 0]];
  const ridges: [Point, Point][] = [];
  const spawn = (parent: Point, angle: number, level: number) => {
    if (level >= settings.levels) return;
    const count = Math.max(Math.floor(random() * 3), level <= 2 ? 2 : 0);
    const delta = (((60 - level * 8) / count) * Math.PI) / 180;
    for (let i = 0; i < count; i++) {
      const a = angle + (i - 0.5) * delta + (random() * 2 - 1) * delta * 0.25;
      const length = 2 * (0.7 + random() * 0.6);
      const child: Point = [parent[0] + Math.cos(a) * length, parent[1] + Math.sin(a) * length];
      nodes.push(child);
      ridges.push([parent, child]);
      spawn(child, a, level + 1);
    }
  };
  spawn(nodes[0], 0, 0);
  // Root on the left edge, farthest node on the right edge; scale grows it past the square.
  const fit = 2 / Math.max(...nodes.flat());
  for (const node of nodes) {
    node[0] = node[0] * fit - 1;
    node[1] *= fit;
  }
  for (let step = 0; step < 100; step++) {
    for (const a of nodes) {
      for (const b of nodes) {
        const dx = b[0] - a[0],
          dz = b[1] - a[1];
        if (dx * dx + dz * dz < 1 / 9) {
          a[0] -= dx * 0.01;
          a[1] -= dz * 0.01;
          b[0] += dx * 0.01;
          b[1] += dz * 0.01;
        }
      }
    }
  }
  const reach = settings.scale;
  for (const node of nodes) {
    node[0] *= reach;
    node[1] *= reach;
  }
  const lattice = Float32Array.from({ length: 128 * 128 }, () => random());
  function noise(x: number, y: number) {
    const ix = Math.floor(x),
      iy = Math.floor(y);
    let u = x - ix,
      v = y - iy;
    u = u * u * (3 - 2 * u);
    v = v * v * (3 - 2 * v);
    const at = (a: number, b: number) => lattice[(a & 127) + (b & 127) * 128];
    return (
      (at(ix, iy) * (1 - u) + at(ix + 1, iy) * u) * (1 - v) +
      (at(ix, iy + 1) * (1 - u) + at(ix + 1, iy + 1) * u) * v
    );
  }
  const angle = (settings.direction * Math.PI) / 180;
  const cs = Math.cos(angle),
    sn = Math.sin(angle);
  const heights = new Float32Array(GRID * GRID);
  for (let j = 0; j < GRID; j++) {
    for (let i = 0; i < GRID; i++) {
      const px = (i / (GRID - 1)) * 2 - 1,
        pz = (j / (GRID - 1)) * 2 - 1;
      let x = px * cs - pz * sn,
        z = px * sn + pz * cs;
      x += (noise(x * 4 + 22, z * 4 + 53) - 0.5) * 0.07;
      z += (noise(x * 4 + 71, z * 4 + 19) - 0.5) * 0.07;
      let distance = Infinity;
      for (const [a, b] of ridges) {
        const dx = b[0] - a[0],
          dz = b[1] - a[1];
        const t = Math.max(
          0,
          Math.min(1, ((x - a[0]) * dx + (z - a[1]) * dz) / (dx * dx + dz * dz)),
        );
        distance = Math.min(distance, Math.hypot(x - a[0] - dx * t, z - a[1] - dz * t));
      }
      // Tallest at the root, easing down to 0.27x toward the far edge.
      const peak = 1 - (11 / 15) * smoothstep(1, 0.2, smoothstep(1, 0, (x / reach + 1) / 2));
      let falloff = distance / (settings.width * (0.7 + peak * 0.3));
      if (settings.rounding > 0 && falloff < settings.rounding) {
        // Zero slope at the crest, matching the original slope at the shoulder.
        const t = falloff / settings.rounding;
        falloff = settings.rounding * t * t * (2 - t);
      }
      const ridge = peak * Math.max(0, 1 - falloff) ** 1.7;
      let detail = 0;
      for (let octave = 0; octave < 5; octave++) {
        const frequency = 6 * 2 ** octave;
        detail += (noise(x * frequency + 30, z * frequency + 30) - 0.5) / 2 ** octave;
      }
      const mountain = Math.max(0, ridge + detail * settings.noise * Math.min(1, ridge * 8));
      const mountainHeight = mountain * settings.height;
      let floor = 0;
      if (settings.noiseFloor > 0 && mountainHeight < 0.6) {
        let nx = px * 1.5 + 43,
          nz = pz * 1.5 + 79;
        for (let octave = 0; octave < 7; octave++) {
          floor += noise(nx, nz) / 2 ** (octave + 1);
          [nx, nz] = [nx * 1.6 - nz * 1.2, nx * 1.2 + nz * 1.6];
        }
        // Positive seeded relief avoids flat clamped valleys; fade it into the foothills.
        const t = mountainHeight / 0.6;
        floor *= settings.noiseFloor * (1 - t * t * (3 - 2 * t));
      }
      heights[i + j * GRID] = ((0.015 + mountainHeight + floor) * (GRID - 1)) / WORLD_SIZE;
    }
  }
  const seeds = Uint32Array.from(
    { length: DROPLETS },
    () => Math.floor(random() * 4294967296) || 1,
  );
  return { heights, seeds };
}
