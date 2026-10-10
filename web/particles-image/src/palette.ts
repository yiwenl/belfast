export type PaletteColor = readonly [number, number, number];
export const PALETTE_SIZE = 8;
const SAMPLE_SIZE = 128;
const MIN_COLOR_DISTANCE = 16 ** 2;
type ColorBin = { color: PaletteColor; count: number };

function distance(a: PaletteColor, b: PaletteColor): number {
  return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2;
}

function brightness(color: PaletteColor): number {
  return color[0] * 0.2126 + color[1] * 0.7152 + color[2] * 0.0722;
}

export function extractPalette(pixels: Uint8ClampedArray, count = PALETTE_SIZE): PaletteColor[] {
  if (pixels.length % 4) throw new Error("Expected RGBA pixels");
  if (!Number.isInteger(count) || count < 1 || count > 16)
    throw new Error("Palette count must be between 1 and 16");

  // A 5-bit histogram bounds the work; retain the actual RGB means in each bin.
  const histogram = new Map<number, { r: number; g: number; b: number; count: number }>();
  for (let i = 0; i < pixels.length; i += 4) {
    if (pixels[i + 3] < 128) continue;
    const r = pixels[i],
      g = pixels[i + 1],
      b = pixels[i + 2];
    const key = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3);
    const bin = histogram.get(key) ?? { r: 0, g: 0, b: 0, count: 0 };
    bin.r += r;
    bin.g += g;
    bin.b += b;
    bin.count++;
    histogram.set(key, bin);
  }
  if (!histogram.size) throw new Error("Image has no visible colors");
  const bins: ColorBin[] = [...histogram.entries()]
    .sort(([a], [b]) => a - b)
    .map(([, bin]) => ({
      color: [bin.r / bin.count, bin.g / bin.count, bin.b / bin.count],
      count: bin.count,
    }));

  // Deterministic, population-weighted seeds favor major colors plus distinct accents.
  const dominant = bins.reduce((best, bin) => (bin.count > best.count ? bin : best));
  let centers: PaletteColor[] = [dominant.color];
  while (centers.length < count) {
    let candidate: ColorBin | undefined;
    let bestScore = 0;
    for (const bin of bins) {
      const separation = Math.min(...centers.map((center) => distance(bin.color, center)));
      const score = separation * bin.count;
      if (separation >= MIN_COLOR_DISTANCE && score > bestScore) {
        bestScore = score;
        candidate = bin;
      }
    }
    if (!candidate) break;
    centers.push(candidate.color);
  }

  for (let iteration = 0; iteration < 12; iteration++) {
    const sums = centers.map(() => ({ r: 0, g: 0, b: 0, count: 0 }));
    for (const bin of bins) {
      let nearest = 0;
      let bestDistance = Infinity;
      centers.forEach((center, index) => {
        const d = distance(bin.color, center);
        if (d < bestDistance) {
          bestDistance = d;
          nearest = index;
        }
      });
      const sum = sums[nearest];
      sum.r += bin.color[0] * bin.count;
      sum.g += bin.color[1] * bin.count;
      sum.b += bin.color[2] * bin.count;
      sum.count += bin.count;
    }
    const next: PaletteColor[] = sums.map((sum, i) =>
      sum.count ? [sum.r / sum.count, sum.g / sum.count, sum.b / sum.count] : centers[i],
    );
    const settled = next.every((color, i) => distance(color, centers[i]) < 0.25);
    centers = next;
    if (settled) break;
  }

  const palette: PaletteColor[] = [];
  for (const center of centers.sort((a, b) => brightness(a) - brightness(b))) {
    const color: PaletteColor = [
      Math.round(center[0]),
      Math.round(center[1]),
      Math.round(center[2]),
    ];
    if (palette.every((other) => distance(color, other) >= MIN_COLOR_DISTANCE)) palette.push(color);
  }
  return palette;
}

// One opaque texel per color gives every swatch equal sampling probability.
export function palettePixels(colors: readonly PaletteColor[]): Uint8ClampedArray<ArrayBuffer> {
  return new Uint8ClampedArray(colors.flatMap(([r, g, b]) => [r, g, b, 255]));
}

export async function createImagePalette(blob: Blob): Promise<{
  colors: PaletteColor[];
  image: ImageData;
}> {
  const bitmap = await createImageBitmap(blob);
  try {
    const scale = Math.min(1, SAMPLE_SIZE / Math.max(bitmap.width, bitmap.height));
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(bitmap.width * scale));
    canvas.height = Math.max(1, Math.round(bitmap.height * scale));
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) throw new Error("Cannot read image colors");
    context.imageSmoothingEnabled = false;
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const colors = extractPalette(context.getImageData(0, 0, canvas.width, canvas.height).data);
    return { colors, image: new ImageData(palettePixels(colors), colors.length, 1) };
  } finally {
    bitmap.close();
  }
}
