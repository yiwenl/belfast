import init, { ExperimentApp } from "../pkg/experiment";
import { Pane } from "tweakpane";
import type { BindingApi } from "@tweakpane/core";
import { bindOrbitInput } from "./input";
import { generateTerrain, GRID } from "./terrain";
import { DEFAULT_HASH, newHash } from "./hash";
import { savePng, saveSvg, download } from "./export";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const counter = document.querySelector<HTMLElement>("#counter")!;
const debugLabel = document.querySelector<HTMLElement>("#debug-label")!;
const url = new URL(location.href);
const values = {
  seed: url.searchParams.get("seed") || DEFAULT_HASH,
  height: 2.8,
  width: 0.34,
  rounding: 0.12,
  branches: 8,
  direction: -12,
  noise: 0.12,
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
  lightYaw: 74,
  lightPitch: 20,
  spotAngle: 0.5,
  spotFalloff: 2,
  contourSpacing: 0.1,
  contourStrength: 0.2,
  shadows: true,
  density: 192,
  contrast: 1,
  lineWidth: 1,
  heightmap: true,
  debugView: 0,
  debugGain: 1.6,
  yaw: 32,
  pitch: 36,
  distance: 21,
  paperWidth: 148,
  paperHeight: 210,
  margin: 10,
  pen: 0.18,
};
let app: ExperimentApp | undefined;
let pane: Pane | undefined;
let detach: (() => void) | undefined;
let frameId = 0,
  disposed = false;
let pendingPng = false,
  pendingSvg = false,
  exporting = false;
let fps = 0,
  frames = 0,
  lastMeter = performance.now();
const cameraBindings: BindingApi[] = [];
let refreshingCamera = false;
const listeners = new AbortController();

function message(text: string) {
  status.textContent = text;
}
function fail(error: unknown) {
  errorOutput.hidden = false;
  errorOutput.textContent = error instanceof Error ? error.message : String(error);
  message("Unable to render");
}
function applyView() {
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
    ]),
  );
  app?.setPaused(values.paused);
  debugLabel.hidden = !values.heightmap;
  debugLabel.textContent = [
    "HEIGHT MAP",
    "INITIAL HEIGHT",
    "HEIGHT CHANGE · WHITE + / BLACK −",
    "DEPOSITED SOIL",
    "TOTAL EROSION",
  ][values.debugView];
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
    ["rounding", "Ridge rounding", 0, 0.3, 0.01],
    ["branches", "Branches", 0, 12, 1],
    ["direction", "Direction", -180, 180, 1],
    ["noise", "Surface noise", 0, 0.35, 0.005],
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

  const simulation = pane.addFolder({ title: "Erosion" });
  simulation.addBinding(values, "paused", { label: "Paused" }).on("change", applyView);
  simulation
    .addBinding(values, "iterations", { label: "Stop at step", min: 100, max: 20000, step: 100 })
    .on("change", applyView);
  simulation
    .addBinding(values, "speed", { label: "Steps / frame", min: 1, max: 16, step: 1 })
    .on("change", applyView);
  simulation.addButton({ title: "Advance 100 steps" }).on("click", () => {
    values.paused = true;
    applyView();
    pane?.refresh();
    app?.advance(100);
  });
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
  render
    .addBinding(values, "lightYaw", { label: "Light azimuth", min: -180, max: 180, step: 1 })
    .on("change", applyView);
  render
    .addBinding(values, "lightPitch", { label: "Light elevation", min: 5, max: 85, step: 1 })
    .on("change", applyView);
  render.addBinding(values, "shadows", { label: "Cast shadows" }).on("change", applyView);
  for (const [key, label, min, max, step] of [
    ["spotAngle", "Spot angle", 0.15, 1.4, 0.01],
    ["spotFalloff", "Spot falloff", 0.1, 8, 0.1],
    ["contourSpacing", "Contour spacing", 0.02, 0.5, 0.01],
    ["contourStrength", "Contour strength", 0, 0.4, 0.01],
  ] as const)
    render.addBinding(values, key, { label, min, max, step }).on("change", applyView);
  render
    .addBinding(values, "density", { label: "Line density", min: 32, max: 320, step: 8 })
    .on("change", applyView);
  render
    .addBinding(values, "contrast", { label: "Line exposure", min: 0.2, max: 2.5, step: 0.05 })
    .on("change", applyView);
  render
    .addBinding(values, "lineWidth", { label: "Preview width", min: 0.5, max: 2, step: 0.1 })
    .on("change", applyView);
  const camera = pane.addFolder({ title: "Camera", expanded: false });
  for (const [key, label, min, max] of [
    ["yaw", "Orbit", -180, 180],
    ["pitch", "Elevation", 5, 89],
    ["distance", "Distance", 6, 40],
  ] as const) {
    cameraBindings.push(
      camera.addBinding(values, key, { label, min, max, step: 0.5 }).on("change", () => {
        if (!refreshingCamera) app?.setCamera(values.yaw, values.pitch, values.distance);
      }),
    );
  }
  camera.addButton({ title: "Reset camera" }).on("click", () => {
    values.yaw = 32;
    values.pitch = 36;
    values.distance = 21;
    app?.resetCamera();
    pane?.refresh();
  });
  const debug = pane.addFolder({ title: "Height map" });
  debug.addBinding(values, "before", { label: "Before erosion" }).on("change", applyView);
  debug.addBinding(values, "heightmap", { label: "Show map" }).on("change", applyView);
  debug
    .addBinding(values, "debugView", {
      label: "Map",
      options: { Height: 0, Initial: 1, "Height change": 2, Deposition: 3, Erosion: 4 },
    })
    .on("change", applyView);
  debug
    .addBinding(values, "debugGain", { label: "Map gain", min: 0.1, max: 10, step: 0.1 })
    .on("change", applyView);
  debug.addButton({ title: "Save height data" }).on("click", async () => {
    if (!app) return;
    try {
      const step = app.steps();
      const settings = { ...values };
      const outputName = filename("json", step, settings.seed);
      const data = (await app.readHeights()) as Float32Array;
      download(
        new Blob(
          [
            JSON.stringify({
              size: GRID,
              worldSize: 12,
              heightUnit: "grid cell",
              channels: ["height", "initial", "depositedSoil", "erosion"],
              seed: settings.seed,
              settings,
              steps: step,
              data: Array.from(data),
            }),
          ],
          { type: "application/json" },
        ),
        outputName,
      );
      message("Height data saved");
    } catch (error) {
      message(String(error));
    }
  });
  const output = pane.addFolder({ title: "Export", expanded: false });
  output.addButton({ title: "Save canvas PNG · ⌘/Ctrl S" }).on("click", capturePng);
  output.addButton({ title: "Export line SVG" }).on("click", captureSvg);
  for (const [key, label, min, max, step] of [
    ["paperWidth", "Paper width mm", 50, 600, 1],
    ["paperHeight", "Paper height mm", 50, 600, 1],
    ["margin", "Margin mm", 0, 20, 1],
    ["pen", "Pen width mm", 0.05, 1, 0.01],
  ] as const)
    output.addBinding(values, key, { label, min, max, step });
}

function dispose() {
  if (disposed) return;
  disposed = true;
  cancelAnimationFrame(frameId);
  listeners.abort();
  detach?.();
  pane?.dispose();
  app?.free();
  app = undefined;
}

async function start() {
  try {
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
    createPane();
    reset();
    function frame(now: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, Math.min(devicePixelRatio, 2));
        created.frame(now);
        frames++;
        if (now - lastMeter > 500) {
          fps = Math.round((frames * 1000) / (now - lastMeter));
          frames = 0;
          lastMeter = now;
          const camera = created.cameraState();
          refreshingCamera = true;
          [values.yaw, values.pitch, values.distance] = camera;
          for (const binding of cameraBindings) binding.refresh();
          refreshingCamera = false;
          const steps = created.steps();
          counter.textContent = `${steps.toLocaleString()} / ${values.iterations.toLocaleString()} STEPS · ${fps} FPS`;
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
          const settings = { ...values, steps: step, camera: Array.from(created.cameraState()) };
          const outputName = filename("svg", step, settings.seed);
          const aspect = Math.floor(canvas.width / 2) / canvas.height;
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
  },
  { signal: listeners.signal },
);
window.addEventListener(
  "pagehide",
  (event) => {
    if (!event.persisted) dispose();
  },
  { signal: listeners.signal },
);
import.meta.hot?.dispose(dispose);
void start();
