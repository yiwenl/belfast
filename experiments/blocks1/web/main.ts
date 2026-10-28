import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { createControls } from "./controls";
import { bindFluidInput } from "./input";
import { bindKineticInput } from "./kinetic-input";
import { createColorDialog } from "./color-dialog";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
let app: ExperimentApp | undefined;
let controls: ReturnType<typeof createControls> | undefined;
let detachInput: (() => void) | undefined;
let kineticInput: ReturnType<typeof bindKineticInput> | undefined;
let colorDialog: ReturnType<typeof createColorDialog> | undefined;
let animationFrame = 0;
let disposed = false;

function dispose() {
  if (disposed) return;
  disposed = true;
  cancelAnimationFrame(animationFrame);
  fps.dispose();
  controls?.dispose();
  detachInput?.();
  kineticInput?.dispose();
  colorDialog?.dispose();
  app?.free();
  app = undefined;
  window.removeEventListener("pagehide", onPageHide);
  window.removeEventListener("keydown", onKeyDown);
}

async function onKeyDown(event: KeyboardEvent) {
  if (
    event.defaultPrevented ||
    event.repeat ||
    event.isComposing ||
    event.ctrlKey ||
    event.metaKey ||
    event.altKey
  )
    return;
  const target = event.target;
  if (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.closest("input, textarea, select"))
  )
    return;

  const key = event.key.toLowerCase();
  if (key === "f") {
    event.preventDefault();
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch (error) {
      console.warn("Could not toggle fullscreen:", error);
    }
  } else if (key === "r") {
    event.preventDefault();
    colorDialog?.generateRandom();
  } else if (key === "d") {
    event.preventDefault();
    controls?.toggleImagePreview();
  }
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
    kineticInput = bindKineticInput(canvas, created);
    colorDialog = createColorDialog(created, (motion) => controls?.syncMotion(motion));
    controls = createControls(
      created,
      () => colorDialog?.open(),
      (visible) => colorDialog?.setImagePreviewVisible(visible),
      kineticInput,
    );
    detachInput = bindFluidInput(canvas, created);
    window.addEventListener("keydown", onKeyDown);

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
