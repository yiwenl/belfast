import init, { ExperimentApp } from "../pkg/experiment";
import { createBenchmark } from "./benchmark";
import { loadBrushMasks } from "./brush-textures";
import { bindColorControls } from "./color-controls";
import { bindLightControls } from "./light-controls";
import { bindFogControls } from "./fog-controls";
import { createFpsMeter } from "./fps";
import { bindOrbitInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import { loadPlyBytes } from "./load-ply";
import type { SortReply, SortRequest } from "./sort-protocol";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const RENDER_WIDTH = 1920;
const RENDER_HEIGHT = 1080;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const sceneInfo = document.querySelector<HTMLElement>("#scene-info")!;
const brush = document.querySelector<HTMLInputElement>("#brush-enabled")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let detachControls: (() => void) | undefined;
let detachColors: (() => void) | undefined;
let detachLights: (() => void) | undefined;
let detachFog: (() => void) | undefined;
let benchmark: ReturnType<typeof createBenchmark> | undefined;
let animationFrame = 0;
let disposed = false;
let sortWorker: Worker | undefined;
let sorterReady = false;
let sceneReady = false;
let gaussianCount = 0;
const loading = new AbortController();

function dispose() {
  if (disposed) return;
  disposed = true;
  loading.abort();
  sortWorker?.terminate();
  cancelAnimationFrame(animationFrame);
  fps.dispose();
  benchmark?.dispose();
  detachInput?.();
  detachControls?.();
  detachColors?.();
  detachLights?.();
  detachFog?.();
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

function bindSceneControls(target: ExperimentApp) {
  const count = document.querySelector<HTMLInputElement>("#particle-count")!;
  const movement = document.querySelector<HTMLInputElement>("#movement-speed")!;
  const blinking = document.querySelector<HTMLInputElement>("#blink-frequency")!;
  const duration = document.querySelector<HTMLInputElement>("#flash-duration")!;
  const movementRandomness = document.querySelector<HTMLInputElement>("#movement-randomness")!;
  const blinkRandomness = document.querySelector<HTMLInputElement>("#blink-randomness")!;
  const size = document.querySelector<HTMLInputElement>("#firefly-size")!;
  const fov = document.querySelector<HTMLInputElement>("#camera-fov")!;
  const countValue = document.querySelector<HTMLOutputElement>("#particle-count-value")!;
  const fireflyInfo = document.querySelector<HTMLElement>("#firefly-info")!;
  const movementValue = document.querySelector<HTMLOutputElement>("#movement-speed-value")!;
  const blinkingValue = document.querySelector<HTMLOutputElement>("#blink-frequency-value")!;
  const durationValue = document.querySelector<HTMLOutputElement>("#flash-duration-value")!;
  const movementRandomnessValue = document.querySelector<HTMLOutputElement>(
    "#movement-randomness-value",
  )!;
  const blinkRandomnessValue =
    document.querySelector<HTMLOutputElement>("#blink-randomness-value")!;
  const sizeValue = document.querySelector<HTMLOutputElement>("#firefly-size-value")!;
  const fovValue = document.querySelector<HTMLOutputElement>("#camera-fov-value")!;
  const updateControls = () => {
    try {
      const activeCount = target.setFireflyCount(count.valueAsNumber);
      target.setFireflyRates(movement.valueAsNumber, blinking.valueAsNumber);
      target.setFireflyFlashDuration(duration.valueAsNumber);
      target.setFireflyRandomness(movementRandomness.valueAsNumber, blinkRandomness.valueAsNumber);
      target.setFireflySize(size.valueAsNumber);
      const cameraFov = target.setCameraFov(fov.valueAsNumber);
      target.setBrushEnabled(brush.checked);
      count.value = String(activeCount);
      countValue.value = `${activeCount.toLocaleString("en-US")} 隻`;
      fireflyInfo.textContent = `${activeCount.toLocaleString("en-US")} fireflies · flocking & synchronized glow`;
      movementValue.value = `${movement.valueAsNumber.toFixed(2)}×`;
      blinkingValue.value = `${blinking.valueAsNumber.toFixed(2)}×`;
      durationValue.value = `${duration.valueAsNumber.toFixed(2)} 秒`;
      movementRandomnessValue.value = `${movementRandomness.valueAsNumber.toFixed(2)}×`;
      blinkRandomnessValue.value = `${blinkRandomness.valueAsNumber.toFixed(2)}×`;
      sizeValue.value = `${size.valueAsNumber.toFixed(2)}×`;
      fov.value = String(cameraFov);
      fovValue.value = `${cameraFov.toFixed(0)}°`;
    } catch (error) {
      reportError(error);
    }
  };
  const inputs = [
    count,
    movement,
    blinking,
    duration,
    movementRandomness,
    blinkRandomness,
    size,
    fov,
    brush,
  ];
  for (const input of inputs) {
    input.disabled = false;
    input.addEventListener("input", updateControls);
  }
  updateControls();
  return () => {
    for (const input of inputs) {
      input.removeEventListener("input", updateControls);
      input.disabled = true;
    }
  };
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
    document.querySelector<HTMLElement>("#display-info")!.textContent = created.hdrEnabled()
      ? "1920 × 1080 · HDR 輸出"
      : "1920 × 1080 · SDR 輸出";
    detachInput = bindOrbitInput(canvas, created);
    detachControls = bindSceneControls(created);
    detachColors = bindColorControls(created, reportError);
    detachLights = bindLightControls(created, reportError);
    detachFog = bindFogControls(created, reportError);
    benchmark = createBenchmark(created, canvas, (locked) => {
      for (const input of document.querySelectorAll<HTMLInputElement | HTMLButtonElement>(
        "#firefly-controls input, #color-reset",
      )) {
        input.disabled = locked;
      }
      if (locked) {
        detachInput?.();
        detachInput = undefined;
      } else if (!disposed) {
        detachInput = bindOrbitInput(canvas, created);
      }
    });

    // Future Kinetic integration: forward each event's poses and performance.now().
    const skeleton = createSkeletonAdapter(created);
    skeleton.clear();

    sortWorker = new Worker(new URL("./sort.worker.ts", import.meta.url), { type: "module" });
    sortWorker.onerror = (event) =>
      reportError(new Error(event.message || "Gaussian sort worker failed"));
    sortWorker.onmessageerror = () =>
      reportError(new Error("Unable to receive Gaussian sort result"));
    sortWorker.onmessage = ({ data }: MessageEvent<SortReply>) => {
      if (disposed) return;
      try {
        if (data.type === "error") throw new Error(data.message);
        if (data.type === "ready") {
          sorterReady = true;
          return;
        }
        created.applySort(data.indices);
        sceneReady = true;
        benchmark?.setReady();
        sceneInfo.textContent = `${gaussianCount.toLocaleString()} splats · ${created.visibleCount().toLocaleString()} in view · sort ${data.milliseconds.toFixed(1)} ms`;
      } catch (error) {
        reportError(error);
      }
    };

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        // Keep the drawing buffer fixed; CSS scales the 16:9 image to the window.
        created.resize(RENDER_WIDTH, RENDER_HEIGHT, 1);
        // Use the current clock so input received earlier in this RAF is not dated in the future.
        if (!benchmark?.tick(timestampMs)) created.frame(performance.now());
        fps.tick(timestampMs);
        if (sorterReady) {
          // Rust allows one pending sort and accumulates small camera movements.
          const camera = created.requestSort();
          if (camera.length) {
            const message: SortRequest = { type: "sort", camera, brushStrokes: brush.checked };
            sortWorker!.postMessage(message, [camera.buffer]);
          }
        }
        status.hidden = sceneReady;
        animationFrame = requestAnimationFrame(frame);
      } catch (error) {
        reportError(error);
      }
    }
    animationFrame = requestAnimationFrame(frame);
    const brushMasks = await loadBrushMasks(loading.signal);
    if (disposed) return;
    const bytes = await loadPlyBytes(
      `${import.meta.env.BASE_URL}forest.ply`,
      loading.signal,
      (received, total) => {
        status.textContent = `Loading forest · ${Math.round((received / total) * 100)}% · ${(total / 1e6).toFixed(0)} MB`;
      },
    );
    if (disposed) return;
    status.textContent = "Preparing forest textures…";
    // Paint the status before the one-time Rust decode and GPU upload.
    await new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
    if (disposed) return;
    gaussianCount = created.loadPly(bytes);
    created.uploadBrushMasks(brushMasks);
    const positions = created.takeSortPositions();
    const message: SortRequest = { type: "init", positions };
    sortWorker.postMessage(message, [positions.buffer]);
    status.textContent = "Sorting the first view…";
  } catch (error) {
    reportError(error);
  }
}

window.addEventListener("pagehide", onPageHide);
import.meta.hot?.dispose(dispose);
void start();
