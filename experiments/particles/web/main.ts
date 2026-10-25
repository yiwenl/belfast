import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { bindOrbitInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import { loadPlyBytes } from "./load-ply";
import type { SortMode, SortReply, SortRequest } from "./sort-protocol";
import { bindSkyControls } from "./sky";
import skyReport from "../assets/courtyard-sky-separation.json";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const fps = createFpsMeter(document.querySelector<HTMLElement>("#fps")!);
const sceneInfo = document.querySelector<HTMLElement>("#scene-info")!;
const shadowDebug = document.querySelector<HTMLElement>("#shadow-debug")!;
const shadowPreview = document.querySelector<HTMLElement>("#shadow-debug-preview")!;
let shadowResizeObserver: ResizeObserver | undefined;
const petalControls = document.querySelector<HTMLElement>("#petal-controls")!;
const petalCountControl = document.querySelector<HTMLSelectElement>("#petal-count")!;
const petalScaleControl = document.querySelector<HTMLInputElement>("#petal-scale")!;
const petalScaleValue = document.querySelector<HTMLOutputElement>("#petal-scale-value")!;
let petalScale = petalScaleControl.valueAsNumber;
const petalInfo = document.querySelector<HTMLElement>("#petal-info")!;
let petalCount = 10_000;
const sortModeControl = document.querySelector<HTMLSelectElement>("#sort-mode")!;
const sortInfo = document.querySelector<HTMLElement>("#sort-info")!;
const sortMessage = document.querySelector<HTMLElement>("#sort-message")!;
let sortMode: SortMode = "normal";
let sortRevision = 0;
const originalTree = new URLSearchParams(window.location.search).get("tree") === "original";
const params = new URLSearchParams(window.location.search);
const originalSky = originalTree || params.get("sky") === "original";
const sceneFile = originalTree
  ? "courtyard.ply"
  : originalSky
    ? "courtyard-no-tree.ply"
    : "courtyard-foreground.ply";
document.querySelector<HTMLElement>("#scene-title")!.textContent = originalTree
  ? "Courtyard · original"
  : "Courtyard · magnolia";
const variantLink = document.querySelector<HTMLAnchorElement>("#scene-variant")!;
const variantUrl = new URL(window.location.href);
if (originalTree) variantUrl.searchParams.delete("tree");
else variantUrl.searchParams.set("tree", "original");
variantLink.href = variantUrl.href;
variantLink.textContent = originalTree ? "View magnolia" : "View original";
const loading = new AbortController();
let sortWorker: Worker | undefined;
let sorterReady = false;
let sceneReady = false;
let gaussianCount = 0;
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let detachSky: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;

function dispose() {
  if (disposed) return;
  disposed = true;
  cancelAnimationFrame(animationFrame);
  loading.abort();
  sortWorker?.terminate();
  fps.dispose();
  detachInput?.();
  detachSky?.();
  shadowResizeObserver?.disconnect();
  sortModeControl.removeEventListener("change", onSortModeChange);
  petalCountControl.removeEventListener("change", onPetalCountChange);
  petalScaleControl.removeEventListener("input", onPetalScaleChange);
  app?.free();
  app = undefined;
  window.removeEventListener("pagehide", onPageHide);
}

function onSortModeChange() {
  const next: SortMode = sortModeControl.value === "once" ? "once" : "normal";
  if (next === sortMode || !app || disposed) return;
  sortMode = next;
  sortRevision += 1;
  app.invalidateSort();
  sortInfo.textContent = "Updating…";
  sortMessage.textContent =
    next === "once" ? "Capturing this view's order…" : "Resuming depth sorting…";
}

sortModeControl.addEventListener("change", onSortModeChange);

async function onPetalCountChange() {
  const next = Number(petalCountControl.value);
  if (next === petalCount || petalCountControl.disabled || !app || disposed) return;
  petalCountControl.disabled = true;
  petalInfo.textContent = "Updating petals…";
  try {
    // Paint feedback before generating and uploading the replacement particle buffers.
    await new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
    if (disposed || !app) return;
    petalCount = app.setPetalCount(next);
    petalInfo.textContent = "GPU simulation";
  } catch (error) {
    petalInfo.textContent = error instanceof Error ? error.message : String(error);
  } finally {
    if (!disposed) {
      petalCountControl.value = String(petalCount);
      petalCountControl.disabled = false;
    }
  }
}

petalCountControl.addEventListener("change", onPetalCountChange);

function onPetalScaleChange() {
  if (petalScaleControl.disabled || !app || disposed) return;
  try {
    petalScale = app.setPetalScale(petalScaleControl.valueAsNumber);
  } catch (error) {
    petalInfo.textContent = error instanceof Error ? error.message : String(error);
  }
  petalScaleControl.value = petalScale.toFixed(2);
  petalScaleValue.value = `${petalScale.toFixed(2)}×`;
}

petalScaleControl.addEventListener("input", onPetalScaleChange);

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
    const updateShadowPreview = () => {
      if (disposed || shadowDebug.hidden) return;
      const rect = shadowPreview.getBoundingClientRect();
      const screen = canvas.getBoundingClientRect();
      if (!screen.width || !screen.height) return;
      created.setShadowDebugRect(
        (rect.left - screen.left) / screen.width,
        (rect.top - screen.top) / screen.height,
        rect.width / screen.width,
        rect.height / screen.height,
      );
    };
    shadowResizeObserver = new ResizeObserver(updateShadowPreview);
    shadowResizeObserver.observe(canvas);
    shadowResizeObserver.observe(shadowPreview);
    detachInput = bindOrbitInput(canvas, created);
    document.querySelector<HTMLElement>("#sky-controls")!.hidden = originalSky;
    const skyCompare = document.querySelector<HTMLAnchorElement>("#sky-compare")!;
    const skyUrl = new URL(window.location.href);
    skyUrl.searchParams.delete("tree");
    if (originalSky) skyUrl.searchParams.delete("sky");
    else skyUrl.searchParams.set("sky", "original");
    skyCompare.href = skyUrl.href;
    skyCompare.textContent = originalSky ? "View separated sky" : "Compare scanned sky";
    if (originalSky) created.setSkyPreset("off");
    else detachSky = bindSkyControls(created, params.get("sky") === "night" ? "night" : "day");
    document.querySelector<HTMLElement>("#sky-info")!.textContent = originalSky
      ? "Scanned sky included in sorting"
      : `${skyReport.active_sky.toLocaleString()} sky splats excluded from sorting`;

    // Future Kinetic integration: forward each event's poses and performance.now().
    // See README.md for the optional camera/tracker setup and disposal.
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
          sortModeControl.disabled = false;
          return;
        }
        if (data.revision !== sortRevision) {
          // Drain the old request before allowing the next one; never display an
          // old mode's result or enqueue a burst of work during rapid toggles.
          created.discardSort();
          return;
        }
        created.applySort(data.indices, data.didSort);
        sceneReady = true;
        status.hidden = true;
        sceneInfo.textContent = `${gaussianCount.toLocaleString()} splats · ${created.visibleCount().toLocaleString()} in view`;
        const work = data.didSort ? "Cull + sort" : "Cull only";
        sortInfo.textContent = `Sort #${data.sortCount} · Cull #${data.cullCount} · ${work}: ${data.milliseconds.toFixed(1)} ms`;
        sortMessage.textContent =
          sortMode === "once"
            ? "Order locked · frustum culling active"
            : "Camera-triggered resort · frustum culling active";
      } catch (error) {
        reportError(error);
      }
    };

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        // Use the current clock so input received earlier in this RAF is not dated in the future.
        created.frame(performance.now());
        fps.tick(timestampMs);
        if (sorterReady) {
          // Rust suppresses requests while one is pending, or below its motion threshold.
          const camera = created.requestSort();
          if (camera.length) {
            const message: SortRequest = {
              type: "sort",
              camera,
              mode: sortMode,
              revision: sortRevision,
            };
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
    async function loadModelAsset(name: string): Promise<Uint8Array | undefined> {
      if (originalTree) return undefined;
      const response = await fetch(`${import.meta.env.BASE_URL}${name}`, {
        signal: loading.signal,
      });
      if (!response.ok) throw new Error(`Unable to load ${name} (${response.status})`);
      return new Uint8Array(await response.arrayBuffer());
    }
    const [bytes, modelBytes, petalBytes, emitterBytes] = await Promise.all([
      loadPlyBytes(`${import.meta.env.BASE_URL}${sceneFile}`, loading.signal, (received, total) => {
        status.textContent = `Loading courtyard · ${Math.round((received / total) * 100)}% · ${(total / 1e6).toFixed(0)} MB`;
      }),
      loadModelAsset("magnolia-branches.glb"),
      loadModelAsset("magnolia-petal.glb"),
      loadModelAsset("magnolia-emitter-positions.json"),
    ]);
    if (disposed) return;
    status.textContent = "Preparing scene textures…";
    // Let the browser paint the status before the one-time Rust decode and upload.
    await new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
    if (disposed) return;
    if (modelBytes) {
      created.loadGlb(modelBytes);
      shadowDebug.hidden = false;
      updateShadowPreview();
    }
    if (petalBytes && emitterBytes) {
      petalCount = created.loadPetals(petalBytes, emitterBytes);
      petalScale = created.setPetalScale(petalScale);
      petalCountControl.value = String(petalCount);
      petalCountControl.disabled = false;
      petalControls.hidden = false;
      petalScaleControl.disabled = false;
    }
    gaussianCount = created.loadPly(bytes);
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
