import { panoramaDimensions } from "./sky-image-header";

export interface SkyApi {
  setSkyPreset(preset: string): void;
  setSkyImage(rgba: Uint8Array, width: number, height: number): void;
}

/** Decode a 2:1 display-RGB panorama; rendering and camera orientation remain in Rust. */
export async function loadSkyPanorama(
  file: Blob,
): Promise<{ rgba: Uint8Array; width: number; height: number }> {
  if (!["image/png", "image/jpeg", "image/webp"].includes(file.type)) {
    throw new Error("Choose a 2:1 PNG, JPEG or WebP panorama.");
  }
  if (file.size > 32 * 1024 * 1024) throw new Error("Choose an image smaller than 32 MB.");
  const [sourceWidth, sourceHeight] = panoramaDimensions(
    new Uint8Array(await file.slice(0, 256 * 1024).arrayBuffer()),
  );
  if (sourceWidth === 0 || sourceWidth !== sourceHeight * 2)
    throw new Error("The panorama must be 2:1 (for example, 4096 × 2048).");
  if (sourceWidth > 8192) throw new Error("Choose a panorama at most 8192 × 4096 pixels.");
  const targetWidth = Math.min(sourceWidth, 4096);
  const bitmap = await createImageBitmap(file, {
    resizeWidth: targetWidth,
    resizeHeight: targetWidth / 2,
    resizeQuality: "high",
  });
  try {
    if (bitmap.width !== bitmap.height * 2)
      throw new Error("The panorama must be 2:1 (for example, 4096 × 2048).");
    const width = Math.min(bitmap.width, 4096);
    const height = width / 2;
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) throw new Error("Unable to decode the sky image.");
    // An opaque background prevents transparent source pixels producing undefined sky colors.
    context.fillStyle = "#000";
    context.fillRect(0, 0, width, height);
    context.drawImage(bitmap, 0, 0, width, height);
    return {
      rgba: new Uint8Array(context.getImageData(0, 0, width, height).data.buffer),
      width,
      height,
    };
  } finally {
    bitmap.close();
  }
}

export function bindSkyControls(api: SkyApi, initial: "day" | "night"): () => void {
  const mode = document.querySelector<HTMLSelectElement>("#sky-mode")!;
  const input = document.querySelector<HTMLInputElement>("#sky-image")!;
  const imageOption = document.querySelector<HTMLOptionElement>("#sky-image-option")!;
  const message = document.querySelector<HTMLElement>("#sky-message")!;
  let generation = 0;
  let disposed = false;
  mode.value = initial;
  api.setSkyPreset(initial);
  function changeMode() {
    generation++;
    api.setSkyPreset(mode.value);
    message.textContent =
      mode.value === "night"
        ? "Night sky only · scene lighting is unchanged"
        : "Background sky · no sorting";
    const url = new URL(window.location.href);
    if (mode.value === "night") url.searchParams.set("sky", "night");
    else url.searchParams.delete("sky");
    history.replaceState(null, "", url);
  }
  async function changeImage() {
    const file = input.files?.[0];
    if (!file) return;
    input.value = "";
    const request = ++generation;
    input.disabled = true;
    message.textContent = "Loading panorama…";
    try {
      const image = await loadSkyPanorama(file);
      if (disposed || request !== generation) return;
      api.setSkyImage(image.rgba, image.width, image.height);
      imageOption.disabled = false;
      mode.value = "image";
      changeMode();
      message.textContent = `${file.name} · background sky · no sorting`;
    } catch (error) {
      if (!disposed && request === generation)
        message.textContent = error instanceof Error ? error.message : String(error);
    } finally {
      if (!disposed) input.disabled = false;
    }
  }
  mode.addEventListener("change", changeMode);
  input.addEventListener("change", changeImage);
  changeMode();
  return () => {
    disposed = true;
    generation++;
    mode.removeEventListener("change", changeMode);
    input.removeEventListener("change", changeImage);
  };
}
