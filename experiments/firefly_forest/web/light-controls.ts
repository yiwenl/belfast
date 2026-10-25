import type { ExperimentApp } from "../pkg/experiment";

export interface LightSettings {
  enabled: boolean;
  strength: number;
  radius: number;
  hz: number; // 0 updates every rendered frame.
}

const input = (id: string) => document.getElementById(id) as HTMLInputElement;

export function lightSettings(): LightSettings {
  return {
    enabled: input("light-enabled").checked,
    strength: input("light-strength").valueAsNumber,
    radius: input("light-radius").valueAsNumber,
    hz: input("light-hz").valueAsNumber === 121 ? 0 : input("light-hz").valueAsNumber,
  };
}

export function validLightSettings(value: unknown): value is LightSettings {
  if (!value || typeof value !== "object") return false;
  const c = value as LightSettings;
  const range = (n: number, min: number, max: number) => Number.isFinite(n) && n >= min && n <= max;
  return (
    typeof c.enabled === "boolean" &&
    range(c.strength, 0, 1) &&
    range(c.radius, 0.25, 2) &&
    (c.hz === 0 || range(c.hz, 1, 120))
  );
}

export function bindLightControls(app: ExperimentApp, onError: (error: unknown) => void) {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>(".lighting input"));
  const listeners = new AbortController();
  const update = () => {
    try {
      const c = lightSettings();
      app.setLightVolume(c.enabled, c.strength, c.radius, c.hz);
      for (const [id, value] of [
        ["light-strength", `${c.strength.toFixed(2)}×`],
        ["light-radius", c.radius.toFixed(2)],
        ["light-hz", c.hz === 0 ? "每幀更新" : `${c.hz.toFixed(0)} 次／秒`],
      ])
        (document.getElementById(`${id}-value`) as HTMLOutputElement).value = value;
      input("light-hz").setAttribute("aria-valuetext", c.hz === 0 ? "每幀更新" : `${c.hz} 次／秒`);
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
