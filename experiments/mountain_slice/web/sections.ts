import { GRID, WORLD_SIZE } from "./terrain";

export const SECTION_SAMPLES = 1024;

// The preview and cut SVG share these terrain-clipped spans, including their ordering.
export function generateSectionLayout(density: number, angle: number, zRange: number) {
  const radians = (angle * Math.PI) / 180;
  const direction = [Math.cos(radians), Math.sin(radians)];
  const half = WORLD_SIZE / 2;
  const bounds = [half, half * zRange];
  const scale = (GRID - 1) / WORLD_SIZE;
  const limit = Math.ceil(density / Math.SQRT2);
  const data: number[] = [];
  for (let slice = -limit; slice <= limit; slice++) {
    const offset = (slice * WORLD_SIZE) / density;
    const origin = [-direction[1] * offset, direction[0] * offset];
    let low = -half * Math.SQRT2;
    let high = half * Math.SQRT2;
    for (let axis = 0; axis < 2; axis++) {
      if (Math.abs(direction[axis]) < 1e-7) {
        if (Math.abs(origin[axis]) > bounds[axis] + 1e-7) high = low;
      } else {
        const a = (-bounds[axis] - origin[axis]) / direction[axis];
        const b = (bounds[axis] - origin[axis]) / direction[axis];
        low = Math.max(low, Math.min(a, b));
        high = Math.min(high, Math.max(a, b));
      }
    }
    if (high - low <= 1e-7) continue;
    // Keep the original square's grid coordinates so cropping never compresses the terrain.
    const point = (t: number) =>
      origin.map((v, axis) =>
        Math.max(0, Math.min(GRID - 1, (v + direction[axis] * t + half) * scale)),
      );
    data.push(...point(low), ...point(high), 0, data.length / 12, 0, 0, low, high, offset, 0);
  }
  return Float32Array.from(data);
}

export function sectionHeight(heights: Float32Array, x: number, z: number, before: boolean) {
  const ix = Math.floor(x),
    iz = Math.floor(z);
  const fx = x - ix,
    fz = z - iz;
  const cell = (cx: number, cz: number) =>
    heights[
      (Math.max(0, Math.min(GRID - 1, cx)) + Math.max(0, Math.min(GRID - 1, cz)) * GRID) * 4 +
        (before ? 1 : 0)
    ];
  const a = cell(ix, iz),
    b = cell(ix + 1, iz),
    c = cell(ix, iz + 1),
    d = cell(ix + 1, iz + 1);
  // Match common.wgsl's triangle interpolation instead of smoothing across the mesh diagonal.
  const height =
    fx + fz <= 1
      ? a * (1 - fx - fz) + b * fx + c * fz
      : d * (fx + fz - 1) + b * (1 - fz) + c * (1 - fx);
  return (height * WORLD_SIZE) / (GRID - 1);
}
