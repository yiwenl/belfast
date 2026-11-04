import { Pane } from "tweakpane";
import init, { ExperimentApp } from "../pkg/experiment";
import { createFpsMeter } from "./fps";
import { bindWalkInput } from "./input";
import { createSkeletonAdapter } from "./kinetic";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("canvas")!;
const status = document.querySelector<HTMLElement>("#status")!;
const errorOutput = document.querySelector<HTMLElement>("#error")!;
let app: ExperimentApp | undefined;
let detachInput: (() => void) | undefined;
let animationFrame = 0;
let disposed = false;
const loading = new AbortController();
const settings = {
  fps: "—",
  resolution: "hd",
  pointSize: 0.01,
  copyOffset: 0.011,
  cameraY: 0.75,
  cameraPitch: 4,
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
display
  .addBinding(settings, "resolution", {
    label: "Resolution",
    options: { "HD · 1920 × 1080": "hd", "Full browser": "full" },
  })
  .on("change", ({ value }) => canvas.classList.toggle("fixed-resolution", value === "hd"));
canvas.classList.toggle("fixed-resolution", settings.resolution === "hd");
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
const light = { forward: 2.9, height: 4.2, intensity: 30, range: 7.6, radius: 1.45 };

const lighting = lightTab.addFolder({ title: "Cylinder light" });
lighting.addBinding(light, "forward", { label: "Forward", min: 0, max: 10, step: 0.1 });
lighting.addBinding(light, "height", { label: "Height", min: 0.1, max: 5, step: 0.1 });
lighting.addBinding(light, "intensity", { label: "Intensity", min: 0, max: 30, step: 0.1 });
lighting.addBinding(light, "range", { label: "Range", min: 0.2, max: 10, step: 0.1 });
lighting.addBinding(light, "radius", { label: "Radius", min: 0.05, max: 5, step: 0.01 });
lighting.on("change", () =>
  app?.setLight(light.forward, light.height, light.intensity, light.range, light.radius),
);
const shadows = { strength: 0.9, showMap: true };
const shadowControls = lightTab.addFolder({ title: "Shadows" });
shadowControls
  .addBinding(shadows, "strength", { label: "Strength", min: 0, max: 1, step: 0.01 })
  .on("change", ({ value }) => app?.setShadowStrength(value));
shadowControls
  .addBinding(shadows, "showMap", { label: "Show shadow map" })
  .on("change", ({ value }) => app?.setShowShadowMap(value));
const particles = {
  count: 250_000,
  visible: true,
  animate: true,
  size: 0.014,
  lifetime: 5,
  lifeVariation: 0.45,
  outsideDecay: 3.8,
  lift: 0.12,
  drag: 1,
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
motionControls.addBinding(particles, "lift", {
  label: "Particle updraft",
  min: 0,
  max: 1,
  step: 0.01,
});
motionControls.addBinding(particles, "drag", { label: "Drag", min: 0.1, max: 8, step: 0.1 });
for (const folder of [particleControls, lifeControls, motionControls]) {
  folder.on("change", updateParticles);
}

const filament = {
  debug: false,
  rings: 12,
  updraft: 0.16,
  drift: 0,
  radius: 1,
  core: 0.031,
  strength: 0.87,
  circulation: 0.047,
  lifetime: 7.4,
};
filamentTab
  .addBinding(filament, "debug", { label: "Debug view" })
  .on("change", ({ value }) => app?.setFilamentDebugVisible(value));
filamentTab
  .addBinding(particles, "groundY", { label: "Ground Y", min: -2, max: 3, step: 0.01 })
  .on("change", updateParticles);
const rings = filamentTab.addFolder({ title: "Rings" });
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
  max: 0.05,
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
    const sourceCount = created.loadPointCloud(bytes);
    document.querySelector<HTMLElement>("#point-count")!.textContent =
      `${sourceCount.toLocaleString()} PLY points + ${sourceCount.toLocaleString()} copies`;
    detachInput = bindWalkInput(canvas, created);
    pane.disabled = false;

    // Retain the template's optional skeleton input for later particle interaction.
    const skeleton = createSkeletonAdapter(created);
    skeleton.clear();

    function frame(timestampMs: number) {
      if (disposed) return;
      try {
        if (settings.resolution === "hd") {
          // Keep the drawing buffer exact while CSS fits the 16:9 image to the window.
          created.resize(1920, 1080, 1);
        } else {
          created.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
        }
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
