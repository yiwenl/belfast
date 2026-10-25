import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { loadFrames } from "./frames";
import { bindOrbitInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
const faceOn = document.querySelector<HTMLInputElement>("#face-on")!;
const showSlices = document.querySelector<HTMLInputElement>("#show-slices")!;
const loading = new AbortController();
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
let framesReady = false;

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
    detachInput = bindOrbitInput(canvas, created);
    const onFaceOn = () => {
      if (faceOn.checked && showSlices.checked) {
        showSlices.checked = false;
        created.setShowSlices(false);
      }
      created.setFaceOn(faceOn.checked);
    };
    const onShowSlices = () => {
      if (showSlices.checked) {
        faceOn.checked = false;
        created.setFaceOn(false);
      }
      created.setShowSlices(showSlices.checked);
    };
    faceOn.addEventListener("change", onFaceOn, { signal: loading.signal });
    showSlices.addEventListener("change", onShowSlices, { signal: loading.signal });
    onFaceOn();

    // Pose storage from the experiment template. This scene does not read it yet.
    const skeleton = createSkeletonAdapter(created);
    skeleton.clear();

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        // Use the current clock so input received earlier in this RAF is not dated in the future.
        created.frame(performance.now());
        fps.tick(timestampMs);
        status.hidden = framesReady;
        animationFrame = requestAnimationFrame(frame);
      } catch (error) {
        reportError(error);
      }
    }
    animationFrame = requestAnimationFrame(frame);
    status.textContent = "Loading frames…";
    await loadFrames(
      (index, pixels) => {
        if (!disposed) created.uploadFrame(index, pixels);
      },
      loading.signal,
      (done, total) => {
        status.textContent = `Loading frames ${done}/${total}`;
      },
    );
    framesReady = !disposed;
  } catch (error) {
    if (disposed || (error instanceof DOMException && error.name === "AbortError")) return;
    reportError(error);
  }
}

window.addEventListener("pagehide", onPageHide);
import.meta.hot?.dispose(dispose);
void start();
