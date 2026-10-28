import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { createControls } from "./controls";
import { bindFluidInput } from "./input";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
let app: ExperimentApp | undefined;
let controls: ReturnType<typeof createControls> | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;

function dispose() {
  if (disposed) return;
  disposed = true;
  cancelAnimationFrame(animationFrame);
  fps.dispose();
  controls?.dispose();
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
    controls = createControls(created);
    detachInput = bindFluidInput(canvas, created);

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        created.resize(
          canvas.clientWidth,
          canvas.clientHeight,
          Math.min(window.devicePixelRatio, 1.5),
        );
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
