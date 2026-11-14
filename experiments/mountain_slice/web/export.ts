import { GRID, WORLD_SIZE } from "./terrain";
import { generateSectionLayout, sectionHeight, SECTION_SAMPLES } from "./sections";

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

export async function saveSectionSvg(
  heights: Float32Array,
  settings: {
    density: number;
    sliceAngle: number;
    baseThickness: number;
    materialThickness: number;
    before: boolean;
    pen: number;
  },
  filename: string,
) {
  if (
    heights.length !== GRID * GRID * 4 ||
    !Number.isFinite(settings.materialThickness) ||
    settings.materialThickness <= 0
  )
    throw new Error("Invalid terrain or material thickness for cut sections.");
  const layout = generateSectionLayout(settings.density, settings.sliceAngle);
  const scale = (settings.materialThickness * settings.density) / WORLD_SIZE;
  const margin = 10;
  const labelHeight = 6;
  let minAlong = Infinity,
    maxAlong = -Infinity;
  for (let i = 0; i < layout.length; i += 12) {
    minAlong = Math.min(minAlong, layout[i + 8]);
    maxAlong = Math.max(maxAlong, layout[i + 9]);
  }
  const width = (maxAlong - minAlong) * scale + 2 * margin;
  const originX = margin - minAlong * scale;
  let row = margin + 12;
  const paths: string[] = [];
  const labels: string[] = [];
  const sections: { id: string; offsetMm: number; startMm: number; endMm: number }[] = [];
  for (let i = 0; i < layout.length; i += 12) {
    const id = `slice-${String(i / 12 + 1).padStart(3, "0")}`;
    const points: Point[] = [];
    let top = 0;
    for (let step = 0; step <= SECTION_SAMPLES; step++) {
      const t = step / SECTION_SAMPLES;
      const x = layout[i] + (layout[i + 2] - layout[i]) * t;
      const z = layout[i + 1] + (layout[i + 3] - layout[i + 1]) * t;
      const height = sectionHeight(heights, x, z, settings.before);
      if (!Number.isFinite(height)) throw new Error("Invalid height in cut section.");
      top = Math.max(top, height);
      points.push([layout[i + 8] + (layout[i + 9] - layout[i + 8]) * t, height]);
    }
    const toPaper = ([x, y]: Point) =>
      `${(originX + x * scale).toFixed(3)} ${(row + labelHeight + (top - y) * scale).toFixed(3)}`;
    const bottom = -settings.baseThickness;
    const outline = [...points, [layout[i + 9], bottom] as Point, [layout[i + 8], bottom] as Point];
    // One closed path per physical piece; no camera, shadow or viewport clipping.
    paths.push(`<path id="${id}" d="M ${outline.map(toPaper).join(" L ")} Z"/>`);
    const offsetMm = layout[i + 10] * scale;
    labels.push(
      `<text x="${margin}" y="${(row + 3).toFixed(3)}">${id} / offset ${offsetMm.toFixed(2)} mm / start ${(layout[i + 8] * scale).toFixed(2)} mm</text>`,
    );
    sections.push({
      id,
      offsetMm,
      startMm: layout[i + 8] * scale,
      endMm: layout[i + 9] * scale,
    });
    row += labelHeight + (top - bottom) * scale + margin;
    if (i % (12 * 8) === 0) await new Promise<void>((resolve) => setTimeout(resolve, 0));
  }
  const metadata = JSON.stringify({
    ...settings,
    modelSideMm: WORLD_SIZE * scale,
    worldUnitMm: scale,
    originXmm: originX,
    stackDirectionXZ: [
      -Math.sin((settings.sliceAngle * Math.PI) / 180),
      Math.cos((settings.sliceAngle * Math.PI) / 180),
    ],
    sections,
  })
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width.toFixed(3)}mm" height="${row.toFixed(3)}mm" viewBox="0 0 ${width.toFixed(3)} ${row.toFixed(3)}">
<metadata>${metadata}</metadata>
<g id="cut-paths" fill="none" stroke="black" stroke-width="${settings.pen}" stroke-linejoin="round">
${paths.join("\n")}
</g>
<g id="labels" fill="black" font-family="monospace" font-size="3">
<text x="${margin}" y="${margin + 3}">Material ${settings.materialThickness} mm / model side ${(WORLD_SIZE * scale).toFixed(2)} mm / print at 100%</text>
${labels.join("\n")}
</g>
</svg>`;
  download(new Blob([svg], { type: "image/svg+xml" }), filename);
  return paths.length;
}
