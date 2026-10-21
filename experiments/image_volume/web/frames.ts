export const FRAME_COUNT = 190;
export const FRAME_SIZE = 1024;

/** Decode every frame to RGBA and hand it to Rust. One canvas, so drawing stays in order. */
export async function loadFrames(
  upload: (index: number, pixels: Uint8Array) => void,
  signal: AbortSignal,
  onProgress: (done: number, total: number) => void,
): Promise<void> {
  const blobs = await Promise.all(
    Array.from({ length: FRAME_COUNT }, async (_, index) => {
      const name = String(index).padStart(3, "0");
      const response = await fetch(`${import.meta.env.BASE_URL}frames/${name}.jpg`, { signal });
      if (!response.ok) throw new Error(`Unable to load frame ${name}`);
      return response.blob();
    }),
  );
  const canvas = new OffscreenCanvas(FRAME_SIZE, FRAME_SIZE);
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("Unable to decode frames");
  for (let index = 0; index < blobs.length; index++) {
    if (signal.aborted) return;
    const bitmap = await createImageBitmap(blobs[index]);
    try {
      if (bitmap.width !== FRAME_SIZE || bitmap.height !== FRAME_SIZE) {
        throw new Error(
          `Frame ${index} is ${bitmap.width}×${bitmap.height}, expected ${FRAME_SIZE}×${FRAME_SIZE}`,
        );
      }
      context.drawImage(bitmap, 0, 0);
    } finally {
      bitmap.close();
    }
    if (signal.aborted) return;
    const rgba = context.getImageData(0, 0, FRAME_SIZE, FRAME_SIZE).data;
    upload(index, new Uint8Array(rgba.buffer, rgba.byteOffset, rgba.byteLength));
    onProgress(index + 1, FRAME_COUNT);
  }
}
