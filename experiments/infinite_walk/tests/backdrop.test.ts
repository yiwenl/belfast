import { test } from "node:test";
import assert from "node:assert/strict";
import { prepareBackdrop } from "../scripts/prepare-backdrop.mjs";

function source(points: number[][]) {
  const fields = ["x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity"];
  const header = Buffer.from(
    `ply\nformat binary_little_endian 1.0\nelement vertex ${points.length}\n${fields.map((f) => `property float ${f}\n`).join("")}end_header\n`,
  );
  const body = Buffer.alloc(points.length * 28);
  points.forEach((p, i) =>
    [...p.slice(0, 3), 0.2, 0.2, 0.2, p[3] ?? 10].forEach((v, k) =>
      body.writeFloatLE(v, i * 28 + k * 4),
    ),
  );
  return Buffer.concat([header, body]);
}

function vertices(bytes: Buffer) {
  const offset = bytes.indexOf("end_header\n") + 11;
  return Array.from({ length: (bytes.length - offset) / 16 }, (_, i) => ({
    xyz: [0, 4, 8].map((k) => bytes.readFloatLE(offset + i * 16 + k)),
    rgb: [...bytes.subarray(offset + i * 16 + 12, offset + i * 16 + 15)],
  }));
}

test("converts both forest banks into a bounded colored cloud with an empty walking corridor", () => {
  const output = prepareBackdrop(
    source([
      [-2, -2, 1],
      [2, -3, 1],
      [2, -2, 1, -100],
      [0, -2, 100],
    ]),
  );
  const points = vertices(output);
  assert.equal(points.length, 2, "transparent and out-of-tile splats must be excluded");
  assert.ok(points.some((p) => p.xyz[0] < -3));
  assert.ok(points.some((p) => p.xyz[0] > 3));
  for (const p of points) {
    assert.ok(p.xyz[1] > 0 && p.xyz[2] >= -9 && p.xyz[2] <= 4.3);
    assert.ok(Math.max(...p.rgb) > Math.min(...p.rgb), "palette must retain color");
  }
});

test("spatial thinning is deterministic and respects the point budget", () => {
  const rows = Array.from({ length: 200 }, (_, i) => [i % 2 ? -2 : 2, -1 - i * 0.02, 0]);
  const input = source([...rows, ...rows]);
  const first = prepareBackdrop(input, 12);
  assert.equal(vertices(first).length, 12);
  assert.deepEqual(prepareBackdrop(input, 12), first);
  assert.equal(vertices(prepareBackdrop(source([rows[0], rows[0]]))).length, 1);
});

test("rejects truncated, non-finite and empty assets before writing output", () => {
  const input = source([[2, -2, 0]]);
  assert.throws(() => prepareBackdrop(input.subarray(0, -1)), /truncated/);
  assert.throws(() => prepareBackdrop(source([[NaN, -2, 0]])), /finite/);
  assert.throws(() => prepareBackdrop(source([[2, -2, 100]])), /no usable/i);
});
