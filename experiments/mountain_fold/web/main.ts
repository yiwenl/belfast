import init, { ExperimentApp } from "../pkg/experiment";
import { Pane, type SliderInputBindingApi } from "tweakpane";
import type { BindingApi } from "@tweakpane/core";
import { bindOrbitInput } from "./input";
import { generateTerrain } from "./terrain";
import { generateVoronoiSegments } from "./voronoi";
import { DEFAULT_HASH, newHash } from "./hash";
import { savePng, saveSvg } from "./export";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const url = new URL(location.href);
const values = {
  seed: url.searchParams.get("seed") || DEFAULT_HASH,
  height: 2.8,
  width: 0.34,
  rounding: 0.12,
  levels: 3,
  direction: -12,
  noise: 0.12,
  noiseFloor: 0.18,
  scale: 1,
  inertia: 0.15,
  capacity: 5,
  erosion: 0.3,
  deposition: 0.22,
  evaporation: 0.015,
  gravity: 4,
  water: 1,
  lifetime: 200,
  settling: 0.2,
  repose: 0.18,
  paused: url.searchParams.get("paused") === "1",
  speed: 6,
  iterations: Number(url.searchParams.get("steps")) || 2400,
  before: false,
  lightYaw: -4,
  lightPitch: 7,
  diffuseGain: 0,
  spotGain: 0,
  roughness: 1,
  envGain: 1,
  metallic: 0,
  hdr: "golden_gate_hills_2k.hdr",
  spotAngle: 0.5,
  spotFalloff: 2,
  contourSpacing: 0.1,
  contourStrength: 0.12,
  minElevation: 0,
  shadows: true,
  density: 192,
  lineMode: 0,
  lineSpacing: 4,
  voronoiCells: 64,
  voronoiSpacing: 0.06,
  voronoiCurve: 0.4,
  contrast: 1,
  invertInk: true,
  lineWidth: 1,
  lineWobble: 0.015,
  wobbleScale: 0.6,
  lineColor: "#99928a",
  contourColor: "#595959",
  meshColor: "#3c3b3a",
  backgroundColor: "#131313",
  svgPreview: false,
  svgDark: false,
  heightmap: false,
  layerSpacing: 8,
  baseThickness: 0.6,
  debugView: 0,
  debugGain: 1.6,
  yaw: 45.64276885986328,
  pitch: 28.36391830444336,
  distance: 30,
  orthographic: true,
  fov: 42,
  orthoBound: 9,
  paperWidth: 148,
  paperHeight: 210,
  margin: 10,
  pen: 0.18,
};
let app: ExperimentApp | undefined;
let pane: Pane | undefined;
let spacingBinding: BindingApi | undefined;
const feedback = { status: "Loading WebGPU…", steps: 0, fps: 0, error: "" };
let detach: (() => void) | undefined;
let frameId = 0,
  disposed = false,
  demo = 0;
let pendingPng = false,
  pendingSvg = false,
  exporting = false;
let frames = 0,
  lastMeter = performance.now();
const cameraBindings: BindingApi[] = [];
let refreshingCamera = false;
let voronoiKey = "";
const environments = [
  "bambanani_sunset_2k.hdr",
  "drackenstein_quarry_2k.hdr",
  "golden_gate_hills_2k.hdr",
  "hay_bales_2k.hdr",
  "ludwikowice_farmland_2k.hdr",
  "waterkloof_farm_2k.hdr",
];
let loadedHdr = "";
const listeners = new AbortController();

function message(text: string) {
  feedback.status = text;
}
function fail(error: unknown) {
  feedback.error = error instanceof Error ? error.message : String(error);
  message("Unable to render");
  console.error(error);
}
function applyView() {
  if (app && values.lineMode === 2) {
    const key = JSON.stringify([
      values.seed,
      values.voronoiCells,
      values.voronoiSpacing,
      values.voronoiCurve,
    ]);
    if (key !== voronoiKey) {
      app.setVoronoiSegments(
        generateVoronoiSegments(
          values.seed,
          values.voronoiCells,
          values.voronoiSpacing,
          values.voronoiCurve,
        ),
      );
      voronoiKey = key;
    }
  }
  const color = Number.parseInt(values.lineColor.slice(1), 16);
  const contour = Number.parseInt(values.contourColor.slice(1), 16);
  const mesh = Number.parseInt(values.meshColor.slice(1), 16);
  const background = Number.parseInt(values.backgroundColor.slice(1), 16);
  document.documentElement.style.backgroundColor = values.backgroundColor;
  app?.setView(
    new Float32Array([
      values.lightYaw,
      values.lightPitch,
      values.contrast,
      values.density,
      values.lineWidth * Math.min(devicePixelRatio, 2),
      +values.shadows,
      +values.heightmap,
      values.debugView,
      values.debugGain,
      +values.before,
      values.height,
      values.speed,
      values.iterations,
      values.spotAngle,
      values.spotFalloff,
      values.contourSpacing,
      values.contourStrength,
      values.layerSpacing,
      ((color >> 16) & 255) / 255,
      ((color >> 8) & 255) / 255,
      (color & 255) / 255,
      ((background >> 16) & 255) / 255,
      ((background >> 8) & 255) / 255,
      (background & 255) / 255,
      values.baseThickness,
      values.lineWobble,
      values.wobbleScale,
      values.lineMode,
      values.lineSpacing,
      ((contour >> 16) & 255) / 255,
      ((contour >> 8) & 255) / 255,
      (contour & 255) / 255,
      ((mesh >> 16) & 255) / 255,
      ((mesh >> 8) & 255) / 255,
      (mesh & 255) / 255,
      values.minElevation,
      +values.invertInk,
      values.diffuseGain,
      values.spotGain,
      values.roughness,
      values.envGain,
      values.metallic,
    ]),
  );
  app?.setPaused(values.paused);
}
function loadEnvironment(file: string) {
  if (!app || loadedHdr === file) return;
  const request = file;
  loadedHdr = file;
  void fetch(`/${file}`)
    .then((response) => {
      if (!response.ok) throw new Error(`HDR map failed to load (${response.status})`);
      return response.arrayBuffer();
    })
    .then((buffer) => {
      if (disposed || values.hdr !== request || !app) return;
      const sun = app.setEnvironment(new Uint8Array(buffer));
      values.lightYaw = sun[0];
      values.lightPitch = sun[1];
      pane?.refresh();
      applyView();
    })
    .catch((error: unknown) => {
      if (loadedHdr === request) loadedHdr = "";
      console.error(error);
    });
}
function applyErosion() {
  app?.setErosion(
    new Float32Array([
      values.inertia,
      values.capacity,
      values.erosion,
      values.deposition,
      values.evaporation,
      values.gravity,
      values.water,
      values.lifetime,
      values.settling,
      values.repose,
    ]),
  );
}
function applyProjection() {
  if (!refreshingCamera) app?.setProjection(values.orthographic, values.fov, values.orthoBound);
}
function applyPaper() {
  document.body.classList.toggle("svg-preview", values.svgPreview);
  document.documentElement.style.setProperty(
    "--paper-ratio",
    String(values.paperWidth / values.paperHeight),
  );
  canvas.setAttribute(
    "aria-label",
    values.svgPreview
      ? `SVG preview: terrain lines on ${values.paperWidth} by ${values.paperHeight} mm paper`
      : "Mountain erosion: terrain lines floating above a shaded mesh in one 3D scene",
  );
  if (!app) return;
  app.setSvgPreview(
    values.svgPreview,
    values.svgDark,
    values.paperWidth,
    values.paperHeight,
    values.margin,
    values.pen,
  );
  const camera = app.cameraState();
  refreshingCamera = true;
  [values.yaw, values.pitch, values.distance] = camera;
  values.orthoBound = camera[6];
  for (const binding of cameraBindings) binding.refresh();
  refreshingCamera = false;
}
function reset() {
  if (!app) return;
  const { heights, seeds } = generateTerrain(values);
  app.resetTerrain(heights, seeds);
  applyView();
  applyErosion();
  const next = new URL(location.href);
  next.searchParams.set("seed", values.seed);
  history.replaceState(null, "", next);
  message("Seeded terrain regenerated");
}
function filename(extension: string, steps: number, seed = values.seed) {
  return `mountain-fold-${seed.replace(/[^a-z0-9_-]/gi, "").slice(0, 18)}-${steps}.${extension}`;
}
function capturePng() {
  pendingPng = true;
}
function captureSvg() {
  if (!exporting) pendingSvg = true;
}

function createPane() {
  pane = new Pane({
    title: "Mountain Fold",
    container: document.querySelector<HTMLElement>("#controls")!,
  });
  const mountain = pane.addFolder({ title: "Mountain range", expanded: false });
  mountain.addBinding(values, "seed", { label: "Seed / fxhash" }).on("change", (event) => {
    if (event.last) reset();
  });
  for (const [key, label, min, max, step] of [
    ["height", "Height", 0.8, 4.5, 0.05],
    ["width", "Ridge width", 0.15, 0.5, 0.01],
    ["scale", "Scale", 0.5, 2, 0.05],
    ["rounding", "Ridge rounding", 0, 0.3, 0.01],
    ["levels", "Branch levels", 1, 5, 1],
    ["direction", "Direction", -180, 180, 1],
    ["noise", "Surface noise", 0, 0.35, 0.005],
    ["noiseFloor", "Noise floor", 0, 0.4, 0.01],
  ] as const)
    mountain.addBinding(values, key, { label, min, max, step }).on("change", (event) => {
      if (event.last) reset();
    });
  mountain.addButton({ title: "New seed" }).on("click", () => {
    values.seed = newHash();
    pane?.refresh();
    reset();
  });
  mountain.addButton({ title: "Reset same seed" }).on("click", reset);

  const scene = pane.addFolder({ title: "Scene" });
  const preview = scene.addBinding(values, "svgPreview", { label: "SVG preview" });
  const darkPaper = scene
    .addBinding(values, "svgDark", { label: "Dark paper", hidden: !values.svgPreview })
    .on("change", applyPaper);
  const layerSpacing = scene
    .addBinding(values, "layerSpacing", { label: "Layer spacing", min: 0, max: 10, step: 0.1 })
    .on("change", applyView);
  spacingBinding = layerSpacing;
  const baseThickness = scene
    .addBinding(values, "baseThickness", { label: "Base thickness", min: 0, max: 6, step: 0.05 })
    .on("change", applyView);
  const background = scene
    .addBinding(values, "backgroundColor", { label: "Background" })
    .on("change", applyView);

  const simulation = pane.addFolder({ title: "Erosion" });
  const physics = simulation.addFolder({ title: "Water & sediment", expanded: false });
  for (const [key, label, min, max, step] of [
    ["inertia", "Inertia", 0, 0.95, 0.01],
    ["capacity", "Carry capacity", 0.1, 12, 0.1],
    ["erosion", "Erode rate", 0, 0.8, 0.01],
    ["deposition", "Deposit rate", 0.01, 1, 0.01],
    ["evaporation", "Evaporation", 0.001, 0.08, 0.001],
    ["gravity", "Gravity", 0.1, 12, 0.1],
    ["water", "Water / drop", 0.1, 3, 0.1],
    ["lifetime", "Max lifetime", 20, 400, 1],
    ["settling", "Soil settling", 0, 0.25, 0.01],
    ["repose", "Soil slope", 0, 0.6, 0.01],
  ] as const)
    physics.addBinding(values, key, { label, min, max, step }).on("change", applyErosion);
  simulation.addButton({ title: "Reset erosion" }).on("click", reset);

  const render = pane.addFolder({ title: "Light & lines" });
  const lineMode = render.addBinding(values, "lineMode", {
    label: "Line render",
    options: { "Terrain curves": 0, "Straight hatching": 1, "Voronoi hatching": 2 },
  });
  render
    .addBinding(values, "lightYaw", { label: "Light azimuth", min: -180, max: 180, step: 1 })
    .on("change", applyView);
  render
    .addBinding(values, "lightPitch", { label: "Light elevation", min: 5, max: 85, step: 1 })
    .on("change", applyView);
  render
    .addBinding(values, "diffuseGain", { label: "Diffuse", min: 0, max: 2, step: 0.05 })
    .on("change", applyView);
  render
    .addBinding(values, "spotGain", { label: "Spot", min: 0, max: 2, step: 0.05 })
    .on("change", applyView);
  render
    .addBinding(values, "roughness", { label: "Roughness", min: 0, max: 1, step: 0.01 })
    .on("change", applyView);
  render
    .addBinding(values, "metallic", { label: "Metallic", min: 0, max: 1, step: 0.01 })
    .on("change", applyView);
  render
    .addBinding(values, "hdr", {
      label: "Environment map",
      options: Object.fromEntries(
        environments.map((file) => [file.replace(/\.hdr$/, "").replaceAll("_", " "), file]),
      ),
    })
    .on("change", (event) => {
      if (event.last) loadEnvironment(values.hdr);
    });
  render
    .addBinding(values, "envGain", { label: "Environment", min: 0, max: 2, step: 0.05 })
    .on("change", applyView);
  for (const [key, label, min, max, step] of [
    ["contourSpacing", "Contour spacing", 0.02, 0.5, 0.01],
    ["contourStrength", "Contour strength", 0, 0.4, 0.01],
  ] as const)
    render.addBinding(values, key, { label, min, max, step }).on("change", applyView);
  render
    .addBinding(values, "minElevation", {
      label: "Min elevation",
      min: 0,
      max: 5,
      step: 0.05,
    })
    .on("change", applyView);
  const meshColor = render
    .addBinding(values, "meshColor", { label: "Material color" })
    .on("change", applyView);
  const contourColor = render
    .addBinding(values, "contourColor", { label: "Contour color" })
    .on("change", applyView);
  const density = render
    .addBinding(values, "density", { label: "Line density", min: 32, max: 320, step: 8 })
    .on("change", applyView);
  const spacing = render
    .addBinding(values, "lineSpacing", {
      label: "Line spacing px",
      min: 1,
      max: 16,
      step: 0.25,
      hidden: true,
    })
    .on("change", applyView);
  render
    .addBinding(values, "contrast", { label: "Line exposure", min: 0.2, max: 2.5, step: 0.05 })
    .on("change", applyView);
  render.addBinding(values, "invertInk", { label: "Ink on light" }).on("change", applyView);
  const lineColor = render
    .addBinding(values, "lineColor", { label: "Line color" })
    .on("change", applyView);
  const wobble = render
    .addBinding(values, "lineWobble", { label: "Hand wobble", min: 0, max: 0.08, step: 0.001 })
    .on("change", applyView);
  const wobbleScale = render
    .addBinding(values, "wobbleScale", { label: "Wobble scale", min: 0.2, max: 2, step: 0.05 })
    .on("change", applyView);
  const voronoi = pane.addFolder({ title: "Voronoi hatching", hidden: true });
  voronoi
    .addBinding(values, "voronoiCells", { label: "Cells", min: 4, max: 256, step: 1 })
    .on("change", (event) => {
      if (event.last) applyView();
    });
  voronoi
    .addBinding(values, "voronoiSpacing", {
      label: "Spacing (world)",
      min: 0.01,
      max: 0.1,
      step: 0.001,
    })
    .on("change", (event) => {
      if (event.last) applyView();
    });
  voronoi
    .addBinding(values, "voronoiCurve", { label: "Curve", min: 0, max: 1.5, step: 0.01 })
    .on("change", (event) => {
      if (event.last) applyView();
    });
  lineMode.on("change", () => {
    density.hidden = values.lineMode !== 0;
    spacing.hidden = values.lineMode !== 1;
    wobble.hidden = values.lineMode === 1;
    wobbleScale.hidden = values.lineMode === 1;
    voronoi.hidden = values.lineMode !== 2;
    applyView();
  });
  const camera = pane.addFolder({ title: "Camera", expanded: false });
  const projection = camera.addBinding(values, "orthographic", { label: "Orthographic" });
  const fov = camera
    .addBinding(values, "fov", {
      label: "FOV (°)",
      min: 15,
      max: 90,
      step: 1,
      hidden: values.orthographic,
    })
    .on("change", applyProjection);
  const bounds = camera
    .addBinding(values, "orthoBound", {
      label: "Bounds ±",
      min: 2,
      max: 30,
      step: 0.1,
      hidden: !values.orthographic,
    })
    .on("change", applyProjection);
  const poseBindings = ([["distance", "Distance", 6, 40]] as const).map(([key, label, min, max]) =>
    camera
      .addBinding(values, key, {
        label,
        min,
        max,
        step: 0.5,
        hidden: key === "distance" && values.orthographic,
      })
      .on("change", () => {
        if (!refreshingCamera) app?.setCamera(values.yaw, values.pitch, values.distance);
      }),
  );
  cameraBindings.push(...poseBindings, bounds);
  preview.on("change", () => {
    (poseBindings[0] as SliderInputBindingApi).max = values.svgPreview ? 200 : 40;
    applyPaper();
    darkPaper.hidden = !values.svgPreview;
    for (const binding of [
      layerSpacing,
      baseThickness,
      background,
      lineColor,
      contourColor,
      meshColor,
    ]) {
      binding.hidden = values.svgPreview;
    }
  });
  projection.on("change", () => {
    fov.hidden = values.orthographic;
    bounds.hidden = !values.orthographic;
    poseBindings[0].hidden = values.orthographic;
    applyProjection();
  });
  camera.addButton({ title: "Reset camera" }).on("click", () => {
    values.yaw = 45.64276885986328;
    values.pitch = 28.36391830444336;
    values.distance = 30;
    values.fov = 42;
    values.orthoBound = 9;
    app?.resetCamera();
    pane?.refresh();
  });
  pane.addButton({ title: "Export line SVG" }).on("click", captureSvg);
  pane.addBinding(feedback, "fps", {
    readonly: true,
    interval: 250,
    label: "FPS",
    format: (value: number) => value.toFixed(0),
  });
}

const ease = (t: number) => t * t * (3 - 2 * t);
const DEMO_YAW = 45.64276885986328;
// Overhead with the lines on the mesh, tilt to 30°, then lift the lines off it.
function playDemo() {
  if (!app) return;
  const token = ++demo;
  if (values.svgPreview) {
    values.svgPreview = false;
    applyPaper();
  }
  app.resetCamera();
  values.orthoBound = 15;
  applyProjection();
  values.layerSpacing = 0;
  applyView();
  spacingBinding?.refresh();
  pane?.refresh();
  app.setCamera(DEMO_YAW, 90, 30);
  const start = performance.now();
  const holdTop = 900;
  const tilt = 2200;
  const lift = 1800;
  // Spacing starts while the camera is still arriving, so the two moves never both stop.
  const overlap = 900;
  const tiltStart = holdTop;
  const liftStart = tiltStart + tilt - overlap;
  const liftEnd = liftStart + lift;
  const span = (time: number, from: number, duration: number) =>
    ease(Math.min(1, Math.max(0, (time - from) / duration)));
  const frame = (now: number) => {
    if (token !== demo || !app) return;
    const t = now - start;
    if (t >= liftEnd) return;
    app.setCamera(DEMO_YAW, 90 + (30 - 90) * span(t, tiltStart, tilt), 30);
    values.layerSpacing = 10 * span(t, liftStart, lift);
    applyView();
    spacingBinding?.refresh();
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}
function dispose() {
  if (disposed) return;
  disposed = true;
  demo++;
  cancelAnimationFrame(frameId);
  listeners.abort();
  detach?.();
  pane?.dispose();
  app?.free();
  app = undefined;
}

async function start() {
  try {
    createPane();
    if (!("gpu" in navigator))
      throw new Error("WebGPU is unavailable. Use a WebGPU browser on localhost or HTTPS.");
    await init();
    if (disposed) return;
    const created = await ExperimentApp.create(canvas);
    if (disposed) {
      created.free();
      return;
    }
    app = created;
    detach = bindOrbitInput(canvas, created);
    reset();
    created.setCamera(values.yaw, values.pitch, values.distance);
    applyProjection();
    applyPaper();
    loadEnvironment(values.hdr);
    function frame(now: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, Math.min(devicePixelRatio, 2));
        created.frame(now);
        frames++;
        if (now - lastMeter > 500) {
          feedback.fps = Math.round((frames * 1000) / (now - lastMeter));
          frames = 0;
          lastMeter = now;
          const camera = created.cameraState();
          refreshingCamera = true;
          [values.yaw, values.pitch, values.distance] = camera;
          values.orthoBound = camera[6];
          for (const binding of cameraBindings) binding.refresh();
          refreshingCamera = false;
          const steps = created.steps();
          feedback.steps = steps;
          message(
            values.before
              ? "Initial terrain · simulation continues unless paused"
              : values.paused
                ? "Paused"
                : steps >= values.iterations
                  ? "Erosion complete"
                  : "Eroding…",
          );
        }
        if (pendingPng) {
          pendingPng = false;
          void savePng(canvas, filename("png", created.steps())).then(
            () => message("PNG saved"),
            (error) => message(String(error)),
          );
        }
        if (pendingSvg && !exporting) {
          pendingSvg = false;
          exporting = true;
          const step = created.steps();
          const camera = Array.from(created.cameraState());
          const settings = { ...values, orthoBound: camera[6], steps: step, camera };
          const outputName = filename("svg", step, settings.seed);
          const aspect = settings.svgPreview
            ? (settings.paperWidth - 2 * settings.margin) /
              (settings.paperHeight - 2 * settings.margin)
            : canvas.width / canvas.height;
          message("Exporting visible lines…");
          void (created.readLines() as Promise<Float32Array>)
            .then((data) =>
              saveSvg(
                data,
                aspect,
                {
                  width: settings.paperWidth,
                  height: settings.paperHeight,
                  margin: settings.margin,
                  pen: settings.pen,
                },
                settings,
                outputName,
              ),
            )
            .then(
              (count) => message(`SVG saved · ${count.toLocaleString()} paths`),
              (error) => message(String(error)),
            )
            .finally(() => {
              exporting = false;
            });
        }
        frameId = requestAnimationFrame(frame);
      } catch (error) {
        fail(error);
      }
    }
    frameId = requestAnimationFrame(frame);
  } catch (error) {
    fail(error);
  }
}
window.addEventListener(
  "keydown",
  (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      capturePng();
    }
    const typing =
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLTextAreaElement ||
      event.target instanceof HTMLSelectElement;
    if (
      event.key.toLowerCase() === "p" &&
      !event.repeat &&
      !event.metaKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !typing
    ) {
      event.preventDefault();
      playDemo();
    }
  },
  { signal: listeners.signal },
);
canvas.addEventListener("pointerdown", () => demo++, { signal: listeners.signal });
canvas.addEventListener("wheel", () => demo++, { signal: listeners.signal });
window.addEventListener(
  "pagehide",
  (event) => {
    if (!event.persisted) dispose();
  },
  { signal: listeners.signal },
);
import.meta.hot?.dispose(dispose);
void start();
