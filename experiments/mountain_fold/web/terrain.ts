import { seededRandom } from "./hash";

export const GRID = 512;
export const DROPLETS = 1024;
export const WORLD_SIZE = 12;
export interface TerrainSettings {
  seed: string;
  height: number;
  width: number;
  rounding: number;
  branches: number;
  direction: number;
  noise: number;
  noiseFloor: number;
}
type Point = [number, number, number];

export function generateTerrain(settings: TerrainSettings) {
  const random = seededRandom(settings.seed);
  const ridges: [Point, Point][] = [];
  const trunk: Point[] = Array.from({ length: 7 }, (_, i) => [
    -0.65 + i * 0.21,
    (random() - 0.5) * 0.18,
    (0.52 + Math.sin((i / 6) * Math.PI) * 0.48) * (0.85 + random() * 0.15),
  ]);
  for (let i = 1; i < trunk.length; i++) ridges.push([trunk[i - 1], trunk[i]]);
  for (let i = 0; i < settings.branches; i++) {
    const root = trunk[1 + (i % 5)];
    const side = i % 2 === 0 ? -1 : 1;
    const end: Point = [
      root[0] + (random() - 0.5) * 0.3,
      root[1] + side * (0.25 + random() * 0.22),
      root[2] * 0.32,
    ];
    const mid: Point = [
      (root[0] + end[0]) * 0.5 + (random() - 0.5) * 0.12,
      (root[1] + end[1]) * 0.5,
      root[2] * 0.7,
    ];
    ridges.push([root, mid], [mid, end]);
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
      let ridge = 0;
      for (const [a, b] of ridges) {
        const dx = b[0] - a[0],
          dz = b[1] - a[1];
        const t = Math.max(
          0,
          Math.min(1, ((x - a[0]) * dx + (z - a[1]) * dz) / (dx * dx + dz * dz)),
        );
        const distance = Math.hypot(x - a[0] - dx * t, z - a[1] - dz * t);
        const peak = a[2] * (1 - t) + b[2] * t;
        let falloff = distance / (settings.width * (0.7 + peak * 0.3));
        if (settings.rounding > 0 && falloff < settings.rounding) {
          // Zero slope at the crest, matching the original slope at the shoulder.
          const t = falloff / settings.rounding;
          falloff = settings.rounding * t * t * (2 - t);
        }
        ridge = Math.max(ridge, peak * Math.max(0, 1 - falloff) ** 1.7);
      }
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
