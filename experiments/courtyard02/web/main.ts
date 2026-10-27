import { Pane } from "tweakpane";
import init, { ExperimentApp } from "../pkg/experiment";
import { bindOrbitInput } from "./input";
import { loadPlyBytes } from "./load-ply";
import type { SortReply, SortRequest } from "./sort-protocol";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const settings = {
  fixedResolution: true,
  splatHideDistance: 3.5,
  showPetal: true,
  petalSize: 0.02,
};
const pane = new Pane({ title: "Courtyard" });
pane.addBinding(settings, "fixedResolution", { label: "1920 × 1080" }).on("change", ({ value }) => {
  canvas.classList.toggle("fixed-resolution", value);
});
canvas.classList.toggle("fixed-resolution", settings.fixedResolution);
pane
  .addBinding(settings, "splatHideDistance", {
    label: "Hide distance",
    min: 2,
    max: 4,
    step: 0.01,
  })
  .on("change", ({ value }) => app?.setSplatHideDistance(value));
const petalControls = pane.addFolder({ title: "Petal" });
petalControls
  .addBinding(settings, "showPetal", { label: "Visible" })
  .on("change", updatePetalPreview);
petalControls
  .addBinding(settings, "petalSize", { label: "Size", min: 0.005, max: 0.05, step: 0.001 })
  .on("change", updatePetalPreview);
const stats = { fps: 0 };
const fpsDisplay = pane.addBinding(stats, "fps", {
  label: "FPS",
  readonly: true,
  interval: 0,
  format: (value: number) => value.toFixed(0),
});
let fpsSampleStart: number | undefined;
let fpsFrames = 0;
const loading = new AbortController();
let app: ExperimentApp | undefined;
let worker: Worker | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
let sorterReady = false;

function updatePetalPreview() {
  app?.setPetalPreview(settings.showPetal, settings.petalSize);
}

function dispose() {
  if (disposed) return;
  disposed = true;
  cancelAnimationFrame(animationFrame);
  loading.abort();
  worker?.terminate();
  detachInput?.();
  pane.dispose();
  app?.free();
  app = undefined;
  document.removeEventListener("visibilitychange", resetFps);
  window.removeEventListener("pagehide", onPageHide);
}

function resetFps() {
  fpsSampleStart = undefined;
  fpsFrames = 0;
  stats.fps = 0;
  fpsDisplay.refresh();
}

function updateFps(timestamp: number) {
  if (document.hidden) return;
  if (fpsSampleStart === undefined) {
    fpsSampleStart = timestamp;
    return;
  }
  fpsFrames += 1;
  const elapsed = timestamp - fpsSampleStart;
  if (elapsed >= 500) {
    stats.fps = (fpsFrames * 1000) / elapsed;
    fpsDisplay.refresh();
    fpsSampleStart = timestamp;
    fpsFrames = 0;
  }
}

function onPageHide(event: PageTransitionEvent) {
  if (!event.persisted) dispose();
}

function reportError(error: unknown) {
  if (disposed) return;
  console.error(error);
  dispose();
}

async function start() {
  try {
    if (!("gpu" in navigator)) {
      throw new Error("WebGPU is unavailable. Use a WebGPU-capable browser on localhost or HTTPS.");
    }
    await init();
    if (disposed) return;
    const created = await ExperimentApp.create(canvas);
    if (disposed) {
      created.free();
      return;
    }
    app = created;
    created.setSplatHideDistance(settings.splatHideDistance);
    updatePetalPreview();
    detachInput = bindOrbitInput(canvas, created);
    worker = new Worker(new URL("./sort.worker.ts", import.meta.url), { type: "module" });
    worker.onerror = (event) =>
      reportError(new Error(event.message || "Gaussian sort worker failed"));
    worker.onmessageerror = () => reportError(new Error("Unable to receive Gaussian sort result"));
    worker.onmessage = ({ data }: MessageEvent<SortReply>) => {
      if (disposed) return;
      try {
        if (data.type === "error") throw new Error(data.message);
        if (data.type === "ready") {
          sorterReady = true;
          return;
        }
        created.applySort(data.indices, data.didSort);
      } catch (error) {
        reportError(error);
      }
    };
    function frame(timestamp: number) {
      if (disposed) return;
      try {
        if (settings.fixedResolution) {
          // Keep the drawing buffer exact; CSS fits the 16:9 image to the window.
          created.resize(1920, 1080, 1);
        } else {
          created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        }
        created.frame(timestamp);
        updateFps(timestamp);
        if (sorterReady) {
          const camera = created.requestSort();
          if (camera.length) {
            const message: SortRequest = { type: "sort", camera, mode: "normal", revision: 0 };
            worker!.postMessage(message, [camera.buffer]);
          }
        }
        animationFrame = requestAnimationFrame(frame);
      } catch (error) {
        reportError(error);
      }
    }
    animationFrame = requestAnimationFrame(frame);
    const [bytes, emitters, petal] = await Promise.all([
      loadPlyBytes(`${import.meta.env.BASE_URL}courtyard1.ply`, loading.signal),
      fetch(`${import.meta.env.BASE_URL}flower-emitters.json`, { signal: loading.signal }).then(
        async (response) => {
          if (!response.ok) throw new Error(`Unable to load flower emitters (${response.status})`);
          return new Uint8Array(await response.arrayBuffer());
        },
      ),
      fetch(`${import.meta.env.BASE_URL}white-petal.mesh.json`, { signal: loading.signal }).then(
        async (response) => {
          if (!response.ok) throw new Error(`Unable to load petal model (${response.status})`);
          return new Uint8Array(await response.arrayBuffer());
        },
      ),
    ]);
    if (disposed) return;
    created.loadPly(bytes);
    created.loadFlowerEmitters(emitters);
    created.loadPetal(petal);
    const positions = created.takeSortPositions();
    const message: SortRequest = { type: "init", positions };
    worker.postMessage(message, [positions.buffer]);
  } catch (error) {
    reportError(error);
  }
}

document.addEventListener("visibilitychange", resetFps);
window.addEventListener("pagehide", onPageHide);
import.meta.hot?.dispose(dispose);
void start();
