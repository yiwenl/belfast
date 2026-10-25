const BRUSH_SIZE = 256;
const files = ["brush004.png", "brush009.png", "brush020.png", "brush025.png"];

/** Decode source alpha once; the renderer supplies the forest's own colors. */
export async function loadBrushMasks(signal: AbortSignal): Promise<Uint8Array> {
  const layers = await Promise.all(
    files.map(async (file) => {
      const response = await fetch(`${import.meta.env.BASE_URL}brushes/${file}`, { signal });
      if (!response.ok) throw new Error(`Unable to load brush texture: ${file}`);
      const bitmap = await createImageBitmap(await response.blob());
      try {
        const canvas = new OffscreenCanvas(BRUSH_SIZE, BRUSH_SIZE);
        const context = canvas.getContext("2d");
        if (!context) throw new Error("Unable to decode brush texture");
        context.translate(BRUSH_SIZE / 2, BRUSH_SIZE / 2);
        context.rotate(-Math.PI / 2);
        // Transparent padding prevents adjacent edge texels from becoming a square stamp.
        context.drawImage(bitmap, -120, -120, 240, 240);
        const rgba = context.getImageData(0, 0, BRUSH_SIZE, BRUSH_SIZE).data;
        return Uint8Array.from({ length: BRUSH_SIZE * BRUSH_SIZE }, (_, i) => rgba[i * 4 + 3]);
      } finally {
        bitmap.close();
      }
    }),
  );
  const pixels = new Uint8Array(BRUSH_SIZE * BRUSH_SIZE * files.length);
  layers.forEach((layer, i) => pixels.set(layer, i * BRUSH_SIZE * BRUSH_SIZE));
  return pixels;
}
