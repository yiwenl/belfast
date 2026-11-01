import { Pane } from "tweakpane";
import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { bindWalkInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
const loading = new AbortController();
const settings = { fps: "—", pointSize: 0.01, cameraY: 0.3, whiteParticles: false, fov: 45 };
const pane = new Pane({ title: "Infinite walk" });
pane.disabled = true;
pane.addBinding(settings, "fps", { label: "FPS", readonly: true, interval: 500 });
pane
  .addBinding(settings, "pointSize", {
    label: "Particle size",
    min: 0.001,
    max: 0.05,
    step: 0.001,
  })
  .on("change", ({ value }) => app?.setPointSize(value));
pane
  .addBinding(settings, "whiteParticles", { label: "White particles" })
  .on("change", ({ value }) => app?.setWhiteParticles(value));
pane
  .addBinding(settings, "cameraY", {
    label: "Camera Y",
    min: -2,
    max: 3,
    step: 0.01,
  })
  .on("change", ({ value }) => app?.setCameraY(value));
pane
  .addBinding(settings, "fov", { label: "FOV (°)", min: 10, max: 120, step: 1 })
  .on("change", ({ value }) => app?.setFov(value));
const light = { forward: 3.5, height: 2, intensity: 12, range: 3.3 };
const lighting = pane.addFolder({ title: "Spotlight" });
lighting.addBinding(light, "forward", { label: "Forward", min: 0, max: 10, step: 0.1 });
lighting.addBinding(light, "height", { label: "Height", min: 0.1, max: 5, step: 0.1 });
lighting.addBinding(light, "intensity", { label: "Intensity", min: 0, max: 30, step: 0.1 });
lighting.addBinding(light, "range", { label: "Range", min: 0.2, max: 10, step: 0.1 });
lighting.on("change", () =>
  app?.setLight(light.forward, light.height, light.intensity, light.range),
);
const fps = createFpsMeter((value) => {
  settings.fps = value === null ? "—" : String(value);
});

function dispose() {
  if (disposed) return;
  disposed = true;
  loading.abort();
  cancelAnimationFrame(animationFrame);
  fps.dispose();
  pane.dispose();
  detachInput?.();
  app?.free();
  app = undefined;
  window.removeEventListener("pagehide", onPageHide);
}

function onPageHide(event: PageTransitionEvent) {
  // A persisted page resumes its existing RAF when restored from the back/forward cache.
  if (!event.persisted) dispose();
}

function reportError(error: unknown) {
  if (disposed) return;
  status.hidden = true;
  errorOutput.hidden = false;
  errorOutput.textContent = error instanceof Error ? error.message : String(error);
  dispose();
}

async function start() {
  try {
    if (!("gpu" in navigator)) {
      throw new Error(
        "WebGPU is unavailable. Open this page in a WebGPU-capable browser using localhost or HTTPS.",
      );
    }
    await init();
    if (disposed) return;
    const created = await ExperimentApp.create(canvas);
    if (disposed) {
      created.free();
      return;
    }
    app = created;
    status.textContent = "Loading point cloud…";
    const response = await fetch(`${import.meta.env.BASE_URL}forest.ply`, {
      signal: loading.signal,
    });
    if (!response.ok) throw new Error(`Unable to load point cloud (${response.status})`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (disposed) return;
    const count = created.loadPointCloud(bytes);
    document.querySelector<HTMLElement>("#point-count")!.textContent =
      `${count.toLocaleString()} points`;
    detachInput = bindWalkInput(canvas, created);
    pane.disabled = false;

    // Retain the template's optional skeleton input for later particle interaction.
    const skeleton = createSkeletonAdapter(created);
    skeleton.clear();

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        // Use the current clock so input received earlier in this RAF is not dated in the future.
        created.frame(performance.now());
        fps.tick(timestampMs);
        status.hidden = true;
        animationFrame = requestAnimationFrame(frame);
      } catch (error) {
        reportError(error);
      }
    }
    animationFrame = requestAnimationFrame(frame);
  } catch (error) {
    reportError(error);
  }
}

window.addEventListener("pagehide", onPageHide);
import.meta.hot?.dispose(dispose);
void start();
