import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

// Convert the local Gaussian forest once; the browser only loads the compact RGB cloud.
export function prepareBackdrop(bytes, maxPoints = 80000) {
  const headerEnd = bytes.indexOf("end_header\n") + 11;
  if (headerEnd < 11 || headerEnd > 65536) throw new Error("Invalid PLY header");
  const header = bytes.subarray(0, headerEnd).toString();
  if (!header.startsWith("ply\nformat binary_little_endian 1.0\n"))
    throw new Error("Expected binary little endian PLY");
  const count = Number(header.match(/element vertex (\d+)\n/)?.[1]);
  const fields = [...header.matchAll(/property float (\w+)\n/g)].map((m) => m[1]);
  const stride = fields.length * 4;
  const offsets = ["x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity"].map(
    (f) => fields.indexOf(f) * 4,
  );
  if (!count || offsets.some((o) => o < 0) || !Number.isInteger(maxPoints) || maxPoints < 1)
    throw new Error("Invalid Gaussian PLY or point budget");
  if (bytes.length < headerEnd + count * stride) throw new Error("PLY body is truncated");
  const palette = [
    [170, 128, 72],
    [130, 119, 146],
    [97, 116, 73],
    [77, 111, 118],
  ];
  const occupied = new Set();
  const points = [];
  let seen = 0,
    seed = 0x4be1;
  for (let i = 0; i < count; i++) {
    const values = offsets.map((o) => bytes.readFloatLE(headerEnd + i * stride + o));
    if (!values.every(Number.isFinite)) throw new Error("PLY contains non-finite values");
    const [sx, sy, sz, r, g, b, opacity] = values;
    const y = -sy + 0.5,
      z = -sz;
    // Keep the ground and trunk bases around Y=-0.11; discard only lower scan outliers.
    if (opacity < 0 || z < -9 || z >= 4.3 || y < -0.15 || y > 7.5 || Math.abs(sx) > 8) continue;
    const x = sx + (sx < 0 ? -3.2 : 3.2) + (0.4 * (4.3 - z)) / 13.3;
    const key = `${Math.floor(x / 0.07)},${Math.floor(y / 0.07)},${Math.floor(z / 0.07)}`;
    if (occupied.has(key)) continue;
    occupied.add(key);
    // Large coherent patches keep the muted colors legible through distance haze.
    const group = Math.floor((Math.sin(sx * 0.9 + z * 0.55) * 0.5 + 0.5) * 3.999);
    const luminance = Math.max(0.35, Math.min(1.15, 0.5 + (0.282095 * (r + g + b)) / 3));
    const rgb = palette[group].map((c) => Math.round(c * luminance));
    const point = [x, y, z, ...rgb];
    seen++;
    if (points.length < maxPoints) points.push(point);
    else {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      const slot = Math.floor((seed / 4294967296) * seen);
      if (slot < maxPoints) points[slot] = point;
    }
  }
  if (!points.length) throw new Error("PLY has no usable forest points");
  const outputHeader = Buffer.from(
    `ply\nformat binary_little_endian 1.0\ncomment derived from firefly_forest; A atmosphere + C palette\nelement vertex ${points.length}\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nproperty uchar alpha\nend_header\n`,
  );
  const body = Buffer.alloc(points.length * 16);
  points.forEach((p, i) => {
    p.slice(0, 3).forEach((v, k) => body.writeFloatLE(v, i * 16 + k * 4));
    body.set([...p.slice(3), 255], i * 16 + 12);
  });
  return Buffer.concat([outputHeader, body]);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const source = new URL("../../firefly_forest/public/forest.ply", import.meta.url);
  const destination = new URL("../public/backdrop.ply", import.meta.url);
  const output = prepareBackdrop(await readFile(source));
  await writeFile(destination, output);
  console.log(
    `Prepared backdrop.ply: ${output.length.toLocaleString()} bytes (80,000-point budget)`,
  );
}
