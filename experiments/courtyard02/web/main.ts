import init, { ExperimentApp } from "../pkg/experiment";
import { bindOrbitInput } from "./input";
import { loadPlyBytes } from "./load-ply";
import type { SortReply, SortRequest } from "./sort-protocol";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const loading = new AbortController();
const markerToggle = document.querySelector<HTMLInputElement>("#flower-markers")!;
const markerInfo = document.querySelector<HTMLElement>("#marker-info")!;
let app: ExperimentApp | undefined;
let worker: Worker | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
let sorterReady = false;

function dispose() {
  if (disposed) return;
  disposed = true;
  cancelAnimationFrame(animationFrame);
  loading.abort();
  worker?.terminate();
  detachInput?.();
  markerToggle.removeEventListener("change", onMarkerToggle);
  app?.free();
  app = undefined;
  window.removeEventListener("pagehide", onPageHide);
}

function onMarkerToggle() {
  app?.setFlowerMarkersVisible(markerToggle.checked);
}
markerToggle.addEventListener("change", onMarkerToggle);

function onPageHide(event: PageTransitionEvent) {
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
        status.hidden = true;
      } catch (error) {
        reportError(error);
      }
    };
    function frame(timestamp: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        created.frame(timestamp);
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
    const [bytes, emitters] = await Promise.all([
      loadPlyBytes(
        `${import.meta.env.BASE_URL}courtyard1.ply`,
        loading.signal,
        (received, total) => {
          status.textContent = `Loading courtyard · ${Math.round((received / total) * 100)}%`;
        },
      ),
      fetch(`${import.meta.env.BASE_URL}flower-emitters.json`, { signal: loading.signal }).then(
        async (response) => {
          if (!response.ok) throw new Error(`Unable to load flower emitters (${response.status})`);
          return new Uint8Array(await response.arrayBuffer());
        },
      ),
    ]);
    if (disposed) return;
    status.textContent = "Preparing scene…";
    await new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
    if (disposed) return;
    created.loadPly(bytes);
    const markerCount = created.loadFlowerEmitters(emitters);
    created.setFlowerMarkersVisible(markerToggle.checked);
    markerInfo.textContent = `${markerCount} flower sources`;
    markerToggle.disabled = false;
    const positions = created.takeSortPositions();
    const message: SortRequest = { type: "init", positions };
    worker.postMessage(message, [positions.buffer]);
    status.textContent = "Sorting the first view…";
  } catch (error) {
    reportError(error);
  }
}

window.addEventListener("pagehide", onPageHide);
import.meta.hot?.dispose(dispose);
void start();
