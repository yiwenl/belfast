import { test } from "node:test";
import assert from "node:assert/strict";
import { panoramaDimensions } from "../web/sky-image-header";
import { loadSkyPanorama } from "../web/sky";

function png(width: number, height: number) {
  const bytes = new Uint8Array(24);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82]);
  const view = new DataView(bytes.buffer);
  view.setUint32(16, width);
  view.setUint32(20, height);
  return bytes;
}

test("rejects oversized, square and malformed panoramas before any bitmap decode", async () => {
  // Node has no bitmap decoder: these errors must come from preflight, before decode.
  await assert.rejects(
    loadSkyPanorama(new Blob([png(32768, 16384)], { type: "image/png" })),
    /8192/,
  );
  await assert.rejects(loadSkyPanorama(new Blob([png(512, 512)], { type: "image/png" })), /2:1/);
  await assert.rejects(loadSkyPanorama(new Blob(["invalid"], { type: "image/png" })), /header/);
  assert.deepEqual(panoramaDimensions(png(8192, 4096)), [8192, 4096]);
});

test("reads JPEG and all three WebP dimension encodings", () => {
  const jpeg = Uint8Array.from([
    255, 216, 255, 224, 0, 4, 0, 0, 255, 192, 0, 11, 8, 0, 32, 0, 64, 1, 1, 17, 0,
  ]);
  assert.deepEqual(panoramaDimensions(jpeg), [64, 32]);
  for (const kind of ["VP8X", "VP8L", "VP8 "]) {
    const bytes = new Uint8Array(30);
    const v = new DataView(bytes.buffer);
    bytes.set(new TextEncoder().encode("RIFF"), 0);
    bytes.set(new TextEncoder().encode("WEBP" + kind), 8);
    v.setUint32(4, 22, true);
    v.setUint32(16, 10, true);
    if (kind === "VP8X") {
      bytes[24] = 63;
      bytes[27] = 31;
    } else if (kind === "VP8L") {
      bytes[20] = 47;
      v.setUint32(21, 63 | (31 << 14), true);
    } else {
      bytes.set([157, 1, 42], 23);
      v.setUint16(26, 64, true);
      v.setUint16(28, 32, true);
    }
    assert.deepEqual(panoramaDimensions(bytes), [64, 32]);
    // The pixel chunk can exceed the bounded header read; only its dimensions are needed.
    v.setUint32(16, 500_000, true);
    assert.deepEqual(panoramaDimensions(bytes), [64, 32]);
  }
});
