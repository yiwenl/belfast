import type { ExperimentApp } from "../pkg/experiment";

export interface FogSettings {
  enabled: boolean;
  density: number;
  speed: number;
  light: number;
  noiseFrequency: number;
}

const input = (id: string) => document.getElementById(id) as HTMLInputElement;

export function fogSettings(): FogSettings {
  return {
    enabled: input("fog-enabled").checked,
    density: input("fog-density").valueAsNumber,
    speed: input("fog-speed").valueAsNumber,
    light: input("fog-light").valueAsNumber,
    noiseFrequency: input("fog-noise-frequency").valueAsNumber,
  };
}

export function validFogSettings(value: unknown): value is FogSettings {
  if (!value || typeof value !== "object") return false;
  const c = value as FogSettings;
  const range = (n: number, max: number) => Number.isFinite(n) && n >= 0 && n <= max;
  return (
    typeof c.enabled === "boolean" &&
    range(c.density, 0.2) &&
    range(c.speed, 2) &&
    range(c.light, 4) &&
    range(c.noiseFrequency, 4) &&
    c.noiseFrequency >= 0.1
  );
}

export function bindFogControls(app: ExperimentApp, onError: (error: unknown) => void) {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>(".fog input"));
  const listeners = new AbortController();
  const update = () => {
    try {
      const c = fogSettings();
      app.setFog(c.enabled, c.density, c.speed, c.light, c.noiseFrequency);
      for (const [id, value] of [
        ["fog-density", c.density.toFixed(3)],
        ["fog-speed", `${c.speed.toFixed(2)}×`],
        ["fog-light", `${c.light.toFixed(2)}×`],
        ["fog-noise-frequency", `${c.noiseFrequency.toFixed(2)}×`],
      ])
        (document.getElementById(`${id}-value`) as HTMLOutputElement).value = value;
    } catch (error) {
      onError(error);
    }
  };
  for (const control of inputs) {
    control.disabled = false;
    control.addEventListener("input", update, { signal: listeners.signal });
  }
  update();
  return () => {
    listeners.abort();
    for (const control of inputs) control.disabled = true;
  };
}
