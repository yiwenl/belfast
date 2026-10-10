import assert from "node:assert/strict";
import { test } from "node:test";
import { extractPalette, palettePixels } from "../src/palette.ts";

test("flat images produce one exact color instead of duplicated swatches", () => {
  const pixels = new Uint8ClampedArray([200, 30, 60, 255, 200, 30, 60, 255]);
  assert.deepEqual(extractPalette(pixels), [[200, 30, 60]]);
});

test("major accents survive a dominant dark background and occupy equal texture space", () => {
  const pixels = new Uint8ClampedArray([
    ...Array.from({ length: 90 }, () => [16, 16, 16, 255]).flat(),
    ...Array.from({ length: 5 }, () => [240, 40, 20, 255]).flat(),
    ...Array.from({ length: 5 }, () => [20, 180, 240, 255]).flat(),
  ]);
  const palette = extractPalette(pixels);
  assert.deepEqual(palette, [
    [16, 16, 16],
    [240, 40, 20],
    [20, 180, 240],
  ]);
  assert.deepEqual(
    [...palettePixels(palette)],
    [16, 16, 16, 255, 240, 40, 20, 255, 20, 180, 240, 255],
  );
});

test("nearby shades combine while distinct colors stay separate", () => {
  const pixels = new Uint8ClampedArray([200, 20, 20, 255, 204, 24, 24, 255, 20, 200, 20, 255]);
  assert.deepEqual(extractPalette(pixels), [
    [202, 22, 22],
    [20, 200, 20],
  ]);
});

test("extraction is deterministic, bounded and ignores transparent pixels", () => {
  const pixels = new Uint8ClampedArray([
    255, 0, 255, 0, 0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255,
    255,
  ]);
  const palette = extractPalette(pixels, 3);
  assert.equal(palette.length, 3);
  assert.deepEqual(extractPalette(pixels, 3), palette);
  assert.deepEqual(extractPalette(new Uint8ClampedArray([255, 0, 255, 0, 25, 50, 75, 255])), [
    [25, 50, 75],
  ]);
});

test("unusable pixels fail instead of replacing the current palette", () => {
  assert.throws(() => extractPalette(new Uint8ClampedArray([0, 0, 0, 0])), /visible/);
  assert.throws(() => extractPalette(new Uint8ClampedArray([1, 2, 3])), /RGBA/);
  assert.throws(() => extractPalette(new Uint8ClampedArray([1, 2, 3, 255]), 0), /count/);
});
