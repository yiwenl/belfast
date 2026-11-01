import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { bindWalkInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
const loading = new AbortController();

function dispose() {
  if (disposed) return;
  disposed = true;
  loading.abort();
  cancelAnimationFrame(animationFrame);
  fps.dispose();
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
