import init, { ExperimentApp } from "../pkg/experiment";
import { bindWalkInput } from "./input";
import { loadPlyBytes } from "./load-ply";
import { VideoScrubber } from "./video-sync";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const comparison = document.querySelector<HTMLElement>("#comparison")!;
const video = document.querySelector<HTMLVideoElement>("#walk-video")!;
const videoStatus = document.querySelector<HTMLElement>("#video-status")!;
const videoSync = new VideoScrubber(video);
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
const loading = new AbortController();
const events = new AbortController();
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;

function dispose() {
  if (disposed) return;
  disposed = true;
  loading.abort();
  events.abort();
  cancelAnimationFrame(animationFrame);
  detachInput?.();
  videoSync.dispose();
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
    created.loadCameras(cameras);
    created.loadPointCloud(bytes);
    detachInput = bindWalkInput(canvas, created, comparison);
    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        created.frame(timestampMs);
        videoSync.setTime(created.cameraTime());
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

function updateVideoStatus() {
  if (video.error) {
    videoStatus.textContent = "Video could not load. Check forest-walk-720p.mp4 in public/.";
    videoStatus.hidden = false;
  } else {
    videoStatus.hidden = video.readyState >= 2;
  }
}
video.addEventListener("loadeddata", updateVideoStatus, { signal: events.signal });
video.addEventListener("error", updateVideoStatus, { signal: events.signal });
updateVideoStatus();
window.addEventListener("pagehide", onPageHide);
import.meta.hot?.dispose(dispose);
void start();
