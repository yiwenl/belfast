import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { bindWalkInput } from "./input";
import { loadPlyBytes } from "./load-ply";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const pointCount = document.querySelector<HTMLElement>("#point-count")!;
const cameraIndex = document.querySelector<HTMLElement>("#camera-index")!;
const size = document.querySelector<HTMLInputElement>("#point-size")!;
const sizeValue = document.querySelector<HTMLOutputElement>("#point-size-value")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
const loading = new AbortController();
const events = new AbortController();
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
let lastIndex = -1;

function dispose() {
  if (disposed) return;
  disposed = true;
  loading.abort();
  events.abort();
  cancelAnimationFrame(animationFrame);
  fps.dispose();
  detachInput?.();
  app?.free();
  app = undefined;
  window.removeEventListener("pagehide", onPageHide);
}
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
      throw new Error(
        "WebGPU is unavailable. Open this page in a WebGPU-capable browser on localhost or HTTPS.",
      );
    }
    status.textContent = "Preparing the forest…";
    await init();
    if (disposed) return;
    const created = await ExperimentApp.create(canvas);
    if (disposed) {
      created.free();
      return;
    }
    app = created;
    const base = import.meta.env.BASE_URL;
    const [bytes, cameras] = await Promise.all([
      loadPlyBytes(base + "forest.ply", loading.signal, (received, total) => {
        if (!disposed)
          status.textContent = `Loading point cloud · ${Math.round((received / total) * 100)}%`;
      }),
      fetch(base + "cameras.json", { signal: loading.signal }).then(async (response) => {
        if (!response.ok)
          throw new Error(`Unable to load cameras (${response.status}). Run prepare:assets first.`);
        return response.text();
      }),
    ]);
    if (disposed) return;
    status.textContent = "Building the point cloud…";
    const camerasCount = created.loadCameras(cameras);
    const count = created.loadPointCloud(bytes);
    pointCount.textContent = count.toLocaleString() + " points";
    created.setPointSize(Number(size.value));
    size.disabled = false;
    size.addEventListener(
      "input",
      () => {
        if (disposed) return;
        try {
          created.setPointSize(Number(size.value));
          sizeValue.value = Number(size.value).toFixed(1) + " px";
        } catch (error) {
          reportError(error);
        }
      },
      { signal: events.signal },
    );
    detachInput = bindWalkInput(canvas, created);
    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        created.frame(timestampMs);
        fps.tick(timestampMs);
        const index = created.cameraIndex();
        if (index !== lastIndex) {
          cameraIndex.textContent = `${index + 1} / ${camerasCount}`;
          lastIndex = index;
        }
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
