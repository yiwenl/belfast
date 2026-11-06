import { Pane } from "tweakpane";
import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { bindPointerInput, bindWalkInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let pointerInput: ReturnType<typeof bindPointerInput> | undefined;
let animationFrame = 0;
let savePngRequested = false;
let disposed = false;
const loading = new AbortController();
const resolutions = {
  hd: [1920, 1080],
  portrait: [1080, 1920],
  full: null,
} as const;
const settings = {
  fps: "—",
  output: "—",
  resolution: "hd" as keyof typeof resolutions,
  pointSize: 0.01,
  copyOffset: 0.011,
  cameraY: 1.24,
  cameraPitch: -13,
  whiteParticles: false,
  fov: 88,
};
const pane = new Pane({ title: "Infinite walk" });
pane.disabled = true;
pane.addBinding(settings, "fps", { label: "FPS", readonly: true, interval: 500 });
const [sceneTab, lightTab, particlesTab, filamentTab] = pane.addTab({
  pages: [{ title: "Scene" }, { title: "Light" }, { title: "Particles" }, { title: "Filament" }],
}).pages;

const display = sceneTab.addFolder({ title: "Display" });
const outputStatus = display.addBinding(settings, "output", { label: "Output", readonly: true });
display
  .addBinding(settings, "resolution", {
    label: "Resolution",
    options: {
      "HD · 1920 × 1080": "hd",
      "Portrait · 1080 × 1920": "portrait",
      "Full browser": "full",
    },
  })
  .on("change", fitCanvas);
function fitCanvas() {
  const size = resolutions[settings.resolution];
  canvas.classList.toggle("fixed-resolution", size !== null);
  if (size) canvas.style.setProperty("--canvas-aspect", String(size[0] / size[1]));
}
fitCanvas();
display
  .addBinding(settings, "whiteParticles", { label: "White particles" })
  .on("change", ({ value }) => app?.setWhiteParticles(value));

const cloud = sceneTab.addFolder({ title: "PLY point cloud" });
cloud
  .addBinding(settings, "pointSize", {
    label: "Particle size",
    min: 0.001,
    max: 0.05,
    step: 0.001,
  })
  .on("change", ({ value }) => app?.setPointSize(value));
cloud
  .addBinding(settings, "copyOffset", { label: "Copy offset", min: 0, max: 0.25, step: 0.001 })
  .on("change", ({ value }) => app?.setCopyOffset(value));

const camera = sceneTab.addFolder({ title: "Camera" });
camera
  .addBinding(settings, "cameraY", {
    label: "Camera Y",
    min: 0,
    max: 3,
    step: 0.01,
  })
  .on("change", ({ value }) => app?.setCameraY(value));
camera
  .addBinding(settings, "cameraPitch", { label: "Camera pitch (°)", min: -85, max: 85, step: 1 })
  .on("change", ({ value }) => app?.setCameraPitch(value));
camera
  .addBinding(settings, "fov", { label: "FOV (°)", min: 10, max: 120, step: 1 })
  .on("change", ({ value }) => app?.setFov(value));
const light = { forward: 2.9, height: 4.2, intensity: 100, range: 7.6, radius: 1.45 };

const lighting = lightTab.addFolder({ title: "Cylinder light" });
lighting.addBinding(light, "forward", { label: "Forward", min: 0, max: 10, step: 0.1 });
lighting.addBinding(light, "height", { label: "Height", min: 0.1, max: 5, step: 0.1 });
lighting.addBinding(light, "intensity", { label: "Intensity", min: 0, max: 300, step: 0.1 });
lighting.addBinding(light, "range", { label: "Range", min: 0.2, max: 10, step: 0.1 });
lighting.addBinding(light, "radius", { label: "Radius", min: 0.05, max: 5, step: 0.01 });
lighting.on("change", () =>
  app?.setLight(light.forward, light.height, light.intensity, light.range, light.radius),
);
const shadows = { strength: 0.97, showMap: false };
const shadowControls = lightTab.addFolder({ title: "Shadows" });
shadowControls
  .addBinding(shadows, "strength", { label: "Strength", min: 0, max: 1, step: 0.01 })
  .on("change", ({ value }) => app?.setShadowStrength(value));
shadowControls
  .addBinding(shadows, "showMap", { label: "Show shadow map" })
  .on("change", ({ value }) => app?.setShowShadowMap(value));
const fog = { density: 0.22, halfHeight: 0.45, noiseFrequency: 16 };
const fogControls = lightTab.addFolder({ title: "Volumetric fog" });
fogControls
  .addBinding(fog, "density", { label: "Density", min: 0, max: 0.5, step: 0.01 })
  .on("change", ({ value }) => app?.setFogDensity(value));
fogControls
  .addBinding(fog, "halfHeight", { label: "Ground half-height", min: 0.05, max: 3, step: 0.05 })
  .on("change", ({ value }) => app?.setFogHalfHeight(value));
fogControls
  .addBinding(fog, "noiseFrequency", { label: "Noise frequency", min: 2, max: 64, step: 1 })
  .on("change", ({ value }) => app?.setFogNoiseFrequency(value));
const particles = {
  count: 250_000,
  visible: true,
  animate: true,
  size: 0.014,
  lifetime: 4.5,
  lifeVariation: 0.45,
  outsideDecay: 3.8,
  lift: 0,
  forceMultiplier: 0.98,
  drag: 3.6,
  gravity: 0.43,
  groundY: -0.11,
};
function updateParticles() {
  app?.setParticles(
    particles.visible,
    particles.animate,
    particles.size,
    particles.lifetime,
    particles.lifeVariation,
    particles.lift,
    particles.drag,
    particles.groundY,
    particles.count,
    particles.outsideDecay,
    particles.forceMultiplier,
    particles.gravity,
  );
}
const particleControls = particlesTab.addFolder({ title: "Drifting particles" });
particleControls.addBinding(particles, "count", { label: "Count", min: 0, max: 500_000, step: 1 });
particleControls.addBinding(particles, "visible", { label: "Visible" });
particleControls.addBinding(particles, "size", {
  label: "Size",
  min: 0.001,
  max: 0.05,
  step: 0.001,
});
const pointer = { radius: 0.66, strength: 7.2 };
const pointerControls = particlesTab.addFolder({ title: "Pointer fluid" });
pointerControls.addBinding(pointer, "radius", { label: "Radius", min: 0.05, max: 3, step: 0.01 });
pointerControls.addBinding(pointer, "strength", { label: "Strength", min: 0, max: 10, step: 0.1 });
pointerControls.on("change", () => app?.setPointerForce(pointer.radius, pointer.strength));
const lifeControls = particlesTab.addFolder({ title: "Lifetime" });
lifeControls.addBinding(particles, "lifetime", {
  label: "Life (s)",
  min: 0.5,
  max: 20,
  step: 0.1,
});
lifeControls.addBinding(particles, "lifeVariation", {
  label: "Life variation",
  min: 0,
  max: 0.9,
  step: 0.05,
});
lifeControls.addBinding(particles, "outsideDecay", {
  label: "Outside decay (×)",
  min: 1,
  max: 10,
  step: 0.1,
});
const motionControls = particlesTab.addFolder({ title: "Motion" });
motionControls.addBinding(particles, "animate", { label: "Animate" });
motionControls.addBinding(particles, "forceMultiplier", {
  label: "Force multiplier",
  min: 0,
  max: 2,
  step: 0.01,
});
motionControls.addBinding(particles, "lift", {
  label: "Particle updraft",
  min: 0,
  max: 1,
  step: 0.01,
});
motionControls.addBinding(particles, "drag", { label: "Fall drag", min: 0.1, max: 8, step: 0.1 });
motionControls.addBinding(particles, "gravity", { label: "Gravity", min: 0, max: 2, step: 0.01 });
for (const folder of [particleControls, lifeControls, motionControls]) {
  folder.on("change", updateParticles);
}

const filament = {
  debug: false,
  autoReset: false,
  resetInterval: 3.9,
  rings: 6,
  updraft: 0.24,
  drift: 0.02,
  radius: 0.54,
  core: 0.136,
  strength: 0.27,
  circulation: 0.154,
  lifetime: 3.3,
};
filamentTab
  .addBinding(filament, "debug", { label: "Debug view" })
  .on("change", ({ value }) => app?.setFilamentDebugVisible(value));
filamentTab
  .addBinding(particles, "groundY", { label: "Ground Y", min: -2, max: 3, step: 0.01 })
  .on("change", updateParticles);
const rings = filamentTab.addFolder({ title: "Rings" });
rings.addBinding(filament, "autoReset", { label: "Interval reset" });
rings.addBinding(filament, "resetInterval", {
  label: "Reset interval (s)",
  min: 0,
  max: 20,
  step: 0.1,
});
rings.addBinding(filament, "rings", { label: "Ring count", min: 1, max: 24, step: 1 });
rings.addBinding(filament, "radius", {
  label: "Ring radius",
  min: 0.05,
  max: 2,
  step: 0.01,
});
rings.addBinding(filament, "core", {
  label: "Core radius",
  min: 0.005,
  max: 0.3,
  step: 0.001,
});
rings.addBinding(filament, "lifetime", {
  label: "Ring life (s)",
  min: 1,
  max: 20,
  step: 0.1,
});
const flow = filamentTab.addFolder({ title: "Flow" });
flow.addBinding(filament, "updraft", { label: "Updraft", min: 0, max: 1.5, step: 0.01 });
flow.addBinding(filament, "drift", { label: "XZ drift", min: 0, max: 0.5, step: 0.01 });
flow.addBinding(filament, "strength", { label: "Vortex", min: 0, max: 5, step: 0.01 });
flow.addBinding(filament, "circulation", {
  label: "Circulation",
  min: 0,
  max: 0.2,
  step: 0.001,
});
function updateFilament() {
  app?.setFilament(
    filament.updraft,
    filament.drift,
    filament.radius,
    filament.core,
    filament.strength,
    filament.circulation,
    filament.lifetime,
    filament.rings,
    filament.autoReset,
    filament.resetInterval,
  );
}
rings.on("change", updateFilament);
flow.on("change", updateFilament);
const fps = createFpsMeter((value) => {
  settings.fps = value === null ? "—" : String(value);
});

function dispose() {
  if (disposed) return;
  disposed = true;
  loading.abort();
  cancelAnimationFrame(animationFrame);
  fps.dispose();
  pane.dispose();
  detachInput?.();
  pointerInput?.dispose();
  app?.free();
  app = undefined;
  window.removeEventListener("pagehide", onPageHide);
}

function onPageHide(event: PageTransitionEvent) {
  // A persisted page resumes its existing RAF when restored from the back/forward cache.
  if (!event.persisted) dispose();
}

async function onKeyDown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.isComposing || event.altKey) return;
  if (event.ctrlKey || event.metaKey) {
    if (!event.shiftKey && event.key.toLowerCase() === "s") {
      event.preventDefault();
      if (!event.repeat) savePngRequested = true;
    }
    return;
  }
  if (event.repeat) return;
  const target = event.target;
  if (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.closest("input, textarea, select"))
  )
    return;

  switch (event.key.toLowerCase()) {
    case "f":
      event.preventDefault();
      try {
        if (document.fullscreenElement === canvas) await document.exitFullscreen();
        else await canvas.requestFullscreen();
      } catch (error) {
        console.warn("Could not toggle canvas fullscreen", error);
      }
      break;
    case "p":
      event.preventDefault();
      app?.toggleAutoBackward();
      break;
  }
}

function downloadCanvasPng() {
  canvas.toBlob((blob) => {
    if (!blob) {
      console.warn("Could not export canvas as PNG");
      return;
    }
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `infinite-walk-${Date.now()}.png`;
    link.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  }, "image/png");
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
    settings.output = created.hdrEnabled() ? "HDR" : "SDR";
    outputStatus.refresh();
    status.textContent = "Loading point cloud…";
    const response = await fetch(`${import.meta.env.BASE_URL}forest.ply`, {
      signal: loading.signal,
    });
    if (!response.ok) throw new Error(`Unable to load point cloud (${response.status})`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (disposed) return;
    created.loadPointCloud(bytes);
    detachInput = bindWalkInput(canvas, created);
    pointerInput = bindPointerInput(canvas, created);
    window.addEventListener("keydown", onKeyDown, { signal: loading.signal });
    pane.disabled = false;

    // Retain the template's optional skeleton input for later particle interaction.
    const skeleton = createSkeletonAdapter(created);
    skeleton.clear();

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        const size = resolutions[settings.resolution];
        if (size) {
          created.resize(size[0], size[1], 1);
        } else {
          created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        }
        pointerInput?.update();
        // Use the current clock so input received earlier in this RAF is not dated in the future.
        const rendered = created.frame(performance.now());
        if (rendered && savePngRequested) {
          savePngRequested = false;
          downloadCanvasPng();
        }
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
