export function download(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

export function savePng(canvas: HTMLCanvasElement, filename: string): Promise<void> {
  // Called immediately after the WebGPU frame, before the browser releases its canvas texture.
  return new Promise((resolve, reject) =>
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("Canvas capture returned no image"));
        return;
      }
      download(blob, filename);
      resolve();
    }, "image/png"),
  );
}

type Point = [number, number];
function clip(a: Point, b: Point): [Point, Point] | null {
  const dx = b[0] - a[0],
    dy = b[1] - a[1];
  let low = 0,
    high = 1;
  const p = [-dx, dx, -dy, dy];
  const q = [a[0] + 1, 1 - a[0], a[1] + 1, 1 - a[1]];
  for (let i = 0; i < 4; i++) {
    if (Math.abs(p[i]) < 1e-12) {
      if (q[i] < 0) return null;
    } else if (p[i] < 0) low = Math.max(low, q[i] / p[i]);
    else high = Math.min(high, q[i] / p[i]);
    if (low > high) return null;
  }
  return [
    [a[0] + low * dx, a[1] + low * dy],
    [a[0] + high * dx, a[1] + high * dy],
  ];
}

export async function saveSvg(
  data: Float32Array,
  aspect: number,
  paper: { width: number; height: number; margin: number; pen: number },
  metadata: unknown,
  filename: string,
) {
  const availableWidth = paper.width - paper.margin * 2;
  const availableHeight = paper.height - paper.margin * 2;
  const height = Math.min(availableHeight, availableWidth / aspect);
  const width = height * aspect;
  const offsetX = (paper.width - width) / 2,
    offsetY = (paper.height - height) / 2;
  const toPaper = (p: Point): Point => [
    offsetX + ((p[0] + 1) * width) / 2,
    offsetY + ((1 - p[1]) * height) / 2,
  ];
  const format = (p: Point) => `${p[0].toFixed(3)} ${p[1].toFixed(3)}`;
  const paths: string[] = [];
  let current = "",
    previous: Point | null = null,
    previousRing = -1;
  for (let i = 0; i < data.length; i += 12) {
    if (data[i + 8] < 0.5) {
      previous = null;
      continue;
    }
    const ring = data[i + 9];
    const endpoints = clip(
      [data[i] / data[i + 3], data[i + 1] / data[i + 3]],
      [data[i + 4] / data[i + 7], data[i + 5] / data[i + 7]],
    );
    if (!endpoints) {
      previous = null;
      continue;
    }
    const [a, b] = endpoints.map(toPaper);
    if (![...a, ...b].every(Number.isFinite) || Math.hypot(b[0] - a[0], b[1] - a[1]) < 0.001)
      continue;
    if (
      !previous ||
      ring !== previousRing ||
      Math.hypot(previous[0] - a[0], previous[1] - a[1]) > 0.003
    ) {
      if (current) paths.push(`<path d="${current}"/>`);
      current = `M ${format(a)}`;
    }
    current += ` L ${format(b)}`;
    previous = b;
    previousRing = ring;
    if (i % (12 * 8192) === 0) await new Promise<void>((resolve) => setTimeout(resolve, 0));
  }
  if (current) paths.push(`<path d="${current}"/>`);
  if (!paths.length)
    throw new Error("No visible lines to export. Adjust the camera or line contrast.");
  const description = JSON.stringify(metadata)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${paper.width}mm" height="${paper.height}mm" viewBox="0 0 ${paper.width} ${paper.height}">\n<metadata>${description}</metadata>\n<g fill="none" stroke="black" stroke-width="${paper.pen}" stroke-linecap="round" stroke-linejoin="round">\n${paths.join("\n")}\n</g>\n</svg>`;
  download(new Blob([svg], { type: "image/svg+xml" }), filename);
  return paths.length;
}
