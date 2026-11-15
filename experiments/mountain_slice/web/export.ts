import { GRID, WORLD_SIZE } from "./terrain";
import {
  generateSectionLayout,
  sectionHeight,
  sectionPrintScale,
  SECTION_SAMPLES,
  CUT_PAPER,
} from "./sections";

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

export async function printSections(
  heights: Float32Array,
  settings: {
    density: number;
    sliceAngle: number;
    zRange: number;
    baseThickness: number;
    before: boolean;
    pen: number;
  },
) {
  if (heights.length !== GRID * GRID * 4) throw new Error("Invalid terrain for cut sections.");
  const layout = generateSectionLayout(settings.density, settings.sliceAngle, settings.zRange);
  const scale = sectionPrintScale(layout);
  const materialThickness = (WORLD_SIZE * scale) / settings.density;
  const { width, height, margin } = CUT_PAPER;
  const labelHeight = 5;
  const gap = 4;
  type Page = { paths: string[]; labels: string[] };
  const pages: Page[] = [{ paths: [], labels: [] }];
  let x = margin,
    y = margin,
    rowHeight = 0;
  for (let i = 0; i < layout.length; i += 12) {
    const number = String(i / 12 + 1).padStart(3, "0");
    const id = `slice-${number}`;
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
    const bottom = -settings.baseThickness;
    const pieceWidth = (layout[i + 9] - layout[i + 8]) * scale;
    const blockWidth = Math.max(pieceWidth, 22);
    const blockHeight = labelHeight + (top - bottom) * scale;
    if (blockHeight > height - margin * 2)
      throw new Error(`${id} is too tall for A4. Reduce Base depth or Mountain height.`);
    // Pack in slice order, keeping the same scale and a cutting gap between every piece.
    if (x + blockWidth > width - margin + 1e-6) {
      x = margin;
      y += rowHeight + gap;
      rowHeight = 0;
    }
    if (y + blockHeight > height - margin + 1e-6) {
      pages.push({ paths: [], labels: [] });
      x = margin;
      y = margin;
      rowHeight = 0;
    }
    const page = pages[pages.length - 1];
    const originX = x - layout[i + 8] * scale;
    const toPaper = ([along, elevation]: Point) =>
      `${(originX + along * scale).toFixed(3)} ${(y + labelHeight + (top - elevation) * scale).toFixed(3)}`;
    const outline = [...points, [layout[i + 9], bottom] as Point, [layout[i + 8], bottom] as Point];
    // One closed path per physical piece; no camera, shadow or viewport clipping.
    page.paths.push(`<path id="${id}" d="M ${outline.map(toPaper).join(" L ")} Z"/>`);
    page.labels.push(
      `<text x="${x.toFixed(3)}" y="${(y + 3).toFixed(3)}">${number} x=${(layout[i + 8] * scale).toFixed(1)}</text>`,
    );
    x += blockWidth + gap;
    rowHeight = Math.max(rowHeight, blockHeight);
    if (i % (12 * 8) === 0) await new Promise<void>((resolve) => setTimeout(resolve, 0));
  }
  const output = document.getElementById("print-pages") ?? document.createElement("div");
  output.id = "print-pages";
  output.innerHTML = pages
    .map(
      (page, index) => `<section class="print-page">
<svg xmlns="http://www.w3.org/2000/svg" width="${width}mm" height="${height}mm" viewBox="0 0 ${width} ${height}">
<g fill="none" stroke="black" stroke-width="${settings.pen}" stroke-linejoin="round">
${page.paths.join("\n")}
</g>
<g fill="black" font-family="monospace" font-size="2.5">
<text x="${margin}" y="${margin / 2}">A4 / page ${index + 1} of ${pages.length} / print at 100%</text>
<text x="${margin}" y="${margin / 2 + 4}">Material ${materialThickness.toFixed(3)} mm / widest slice ${(width * 0.8).toFixed(1)} mm</text>
${page.labels.join("\n")}
</g>
</svg>
</section>`,
    )
    .join("\n");
  document.body.append(output);
  document.body.classList.add("print-sections");
  window.print();
  return { sections: layout.length / 12, pages: pages.length };
}
