import { Pane } from "tweakpane";
import type { ExperimentApp } from "../pkg/experiment";
import type { MotionProfile } from "./motion-client";
import type { bindKineticInput } from "./kinetic-input";

export function createControls(
  app: ExperimentApp,
  openColorDialog: () => void,
  setImagePreviewVisible: (visible: boolean) => void,
  kinetic: ReturnType<typeof bindKineticInput>,
) {
  const values = {
    particleCount: 65_536,
    flowStrength: 14,
    flowRadius: 1,
    dissipation: 0.985,
    pressureIterations: 20,
    response: 1,
    depthMotion: 2,
    blockSize: 1,
    shadowStrength: 0.75,
    mouseStrength: 50,
    mouseRadius: 0.08,
    handTracking: true,
    handStrength: 70,
    handRadius: 0.2,
    paused: false,
    debugMaps: false,
    promptMotion: true,
    imagePreview: false,
  };
  const debugLabels = document.querySelector<HTMLElement>("#fluid-debug-labels")!;
  const pane = new Pane({ title: "Blocks 1", expanded: window.innerWidth > 600 });
  pane.addButton({ title: "Generate scene…" }).on("click", openColorDialog);
  const imagePreview = pane.addBinding(values, "imagePreview", { label: "Image preview (D)" });
  const apply = () => {
    app.setSettings(
      new Float32Array([
        values.flowStrength,
        values.flowRadius,
        values.dissipation,
        values.pressureIterations,
        values.response,
        values.depthMotion,
        values.blockSize,
        values.shadowStrength,
        values.mouseStrength,
        values.mouseRadius,
        values.handStrength,
        values.handRadius,
      ]),
    );
    app.setPaused(values.paused);
    app.setParticleCount(values.particleCount);
    app.setDebugMaps(values.debugMaps);
    app.setMotionEnabled(values.promptMotion);
    debugLabels.hidden = !values.debugMaps;
    setImagePreviewVisible(values.imagePreview);
    kinetic.setEnabled(values.handTracking);
  };
  const fluid = pane.addFolder({ title: "Fluid" });
  fluid.addBinding(values, "flowStrength", { label: "Flow strength", min: 0, max: 40, step: 0.1 });
  fluid.addBinding(values, "flowRadius", { label: "Flow radius", min: 0.25, max: 2, step: 0.01 });
  fluid.addBinding(values, "dissipation", { label: "Retention", min: 0.9, max: 1, step: 0.001 });
  fluid.addBinding(values, "pressureIterations", {
    label: "Pressure steps",
    min: 1,
    max: 80,
    step: 1,
  });
  const blocks = pane.addFolder({ title: "Blocks" });
  blocks.addBinding(values, "particleCount", {
    label: "Particle count",
    min: 1,
    max: 500_000,
    step: 1,
  });
  fluid.addBinding(values, "debugMaps", { label: "Debug maps" });
  blocks.addBinding(values, "response", { label: "Flow response", min: 0, max: 3, step: 0.01 });
  blocks.addBinding(values, "depthMotion", { label: "Depth motion", min: 0, max: 3, step: 0.01 });
  blocks.addBinding(values, "blockSize", { label: "Size", min: 0.2, max: 3, step: 0.01 });
  blocks.addBinding(values, "shadowStrength", { label: "Shadows", min: 0, max: 1, step: 0.01 });
  blocks.addBinding(values, "promptMotion", { label: "Prompt motion" });
  const motionValues = {
    type: "manual",
    baseStrength: 0,
    peakStrength: 0,
    intervalSeconds: 0,
    durationSeconds: 0,
    get gain() {
      return app.motionGain();
    },
    get flowStrength() {
      return values.flowStrength * app.motionGain();
    },
    get response() {
      return values.response * app.motionGain();
    },
    get depthMotion() {
      return values.depthMotion * app.motionGain();
    },
  };
  const motion = pane.addFolder({ title: "Motion API", hidden: true });
  const monitor = { readonly: true, interval: 100 } as const;
  const motionBindings = [
    motion.addBinding(motionValues, "type", { ...monitor, label: "Type" }),
    motion.addBinding(motionValues, "baseStrength", { ...monitor, label: "Base strength" }),
    motion.addBinding(motionValues, "peakStrength", { ...monitor, label: "Peak strength" }),
    motion.addBinding(motionValues, "intervalSeconds", { ...monitor, label: "Interval (s)" }),
    motion.addBinding(motionValues, "durationSeconds", { ...monitor, label: "Duration (s)" }),
    motion.addBinding(motionValues, "gain", { ...monitor, label: "Live multiplier" }),
    motion.addBinding(motionValues, "flowStrength", { ...monitor, label: "Live flow" }),
    motion.addBinding(motionValues, "response", { ...monitor, label: "Live response" }),
    motion.addBinding(motionValues, "depthMotion", { ...monitor, label: "Live depth" }),
  ];
  const interaction = pane.addFolder({ title: "Mouse" });
  interaction.addBinding(values, "mouseStrength", { label: "Strength", min: 0, max: 200, step: 1 });
  interaction.addBinding(values, "mouseRadius", {
    label: "Radius",
    min: 0.01,
    max: 0.3,
    step: 0.005,
  });
  pane.addBinding(values, "paused", { label: "Paused" });
  const hands = pane.addFolder({ title: "Kinetic hands" });
  hands.addBinding(values, "handTracking", { label: "Tracking" });
  hands.addBinding(values, "handStrength", { label: "Strength", min: 0, max: 200, step: 1 });
  hands.addBinding(values, "handRadius", { label: "Radius", min: 0.01, max: 0.3, step: 0.005 });
  const handMonitors = [
    hands.addBinding(kinetic.status, "connection", { readonly: true, label: "Status" }),
    hands.addBinding(kinetic.status, "hands", { readonly: true, label: "Tracked hands" }),
  ];
  pane.addButton({ title: "Reset simulation" }).on("click", () => app.reset());
  pane.on("change", (event) => {
    // Monitor refreshes must not feed the effective values back into simulation settings.
    if (![...motionBindings, ...handMonitors].some((binding) => binding === event.target)) apply();
  });
  apply();
  return {
    syncMotion(profile: MotionProfile) {
      Object.assign(motionValues, profile);
      motion.hidden = false;
      for (const binding of motionBindings) binding.refresh();
    },
    toggleImagePreview() {
      values.imagePreview = !values.imagePreview;
      setImagePreviewVisible(values.imagePreview);
      imagePreview.refresh();
    },
    dispose() {
      debugLabels.hidden = true;
      pane.dispose();
    },
  };
}
