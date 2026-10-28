import { Pane } from "tweakpane";
import type { ExperimentApp } from "../pkg/experiment";

export function createControls(app: ExperimentApp, openColorDialog: () => void) {
  const values = {
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
    paused: false,
    debugMaps: false,
    promptMotion: true,
  };
  const debugLabels = document.querySelector<HTMLElement>("#fluid-debug-labels")!;
  const pane = new Pane({ title: "Blocks 1", expanded: window.innerWidth > 600 });
  pane.addButton({ title: "Generate scene…" }).on("click", openColorDialog);
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
      ]),
    );
    app.setPaused(values.paused);
    app.setDebugMaps(values.debugMaps);
    app.setMotionEnabled(values.promptMotion);
    debugLabels.hidden = !values.debugMaps;
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
  fluid.addBinding(values, "debugMaps", { label: "Debug maps" });
  blocks.addBinding(values, "response", { label: "Flow response", min: 0, max: 3, step: 0.01 });
  blocks.addBinding(values, "depthMotion", { label: "Depth motion", min: 0, max: 3, step: 0.01 });
  blocks.addBinding(values, "blockSize", { label: "Size", min: 0.2, max: 3, step: 0.01 });
  blocks.addBinding(values, "shadowStrength", { label: "Shadows", min: 0, max: 1, step: 0.01 });
  blocks.addBinding(values, "promptMotion", { label: "Prompt motion" });
  const interaction = pane.addFolder({ title: "Mouse" });
  interaction.addBinding(values, "mouseStrength", { label: "Strength", min: 0, max: 200, step: 1 });
  interaction.addBinding(values, "mouseRadius", {
    label: "Radius",
    min: 0.01,
    max: 0.3,
    step: 0.005,
  });
  pane.addBinding(values, "paused", { label: "Paused" });
  pane.addButton({ title: "Reset simulation" }).on("click", () => app.reset());
  pane.on("change", apply);
  apply();
  return {
    dispose() {
      debugLabels.hidden = true;
      pane.dispose();
    },
  };
}
