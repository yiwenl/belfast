import type { ExperimentApp } from "../pkg/experiment";

export interface ColorSettings {
  strength: number;
  brightness: number;
  shadow: string;
  highlight: string;
  foliage: string;
  lime: string;
  yellow: string;
  amber: string;
  warmShare: number;
  minPeak: number;
  maxPeak: number;
}

const input = (id: string) => document.getElementById(id) as HTMLInputElement;

export function colorSettings(): ColorSettings {
  return {
    strength: input("grade-strength").valueAsNumber,
    brightness: input("forest-brightness").valueAsNumber,
    shadow: input("forest-shadow").value,
    highlight: input("forest-highlight").value,
    foliage: input("forest-foliage").value,
    lime: input("firefly-lime").value,
    yellow: input("firefly-yellow").value,
    amber: input("firefly-amber").value,
    warmShare: input("firefly-warm-share").valueAsNumber,
    minPeak: input("firefly-peak-min").valueAsNumber,
    maxPeak: input("firefly-peak-max").valueAsNumber,
  };
}

export function validColorSettings(value: unknown): value is ColorSettings {
  if (!value || typeof value !== "object") return false;
  const c = value as ColorSettings;
  const range = (n: number, max: number) => Number.isFinite(n) && n >= 0 && n <= max;
  return (
    range(c.strength, 1) &&
    range(c.brightness, 2) &&
    range(c.warmShare, 1) &&
    range(c.minPeak, 3) &&
    range(c.maxPeak, 3) &&
    c.minPeak <= c.maxPeak &&
    [c.shadow, c.highlight, c.foliage, c.lime, c.yellow, c.amber].every(
      (color) => typeof color === "string" && /^#[0-9a-f]{6}$/.test(color),
    )
  );
}

const rgb = (hex: string) =>
  [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16) / 255);

export function bindColorControls(app: ExperimentApp, onError: (error: unknown) => void) {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>(".appearance input"));
  const reset = document.getElementById("color-reset") as HTMLButtonElement;
  const min = input("firefly-peak-min");
  const max = input("firefly-peak-max");
  const listeners = new AbortController();
  const update = (event?: Event) => {
    try {
      if (min.valueAsNumber > max.valueAsNumber) {
        if (event?.target === min) max.value = min.value;
        else min.value = max.value;
      }
      const c = colorSettings();
      if (!validColorSettings(c)) throw new Error("調色數值無效");
      app.setForestGrade(
        c.strength,
        c.brightness,
        new Float32Array([c.shadow, c.highlight, c.foliage].flatMap(rgb)),
      );
      app.setFireflyAppearance(
        c.minPeak,
        c.maxPeak,
        c.warmShare,
        new Float32Array([c.lime, c.yellow, c.amber].flatMap(rgb)),
      );
      for (const control of inputs) {
        const output = document.getElementById(`${control.id}-value`) as HTMLOutputElement | null;
        if (output)
          output.value =
            control.dataset.unit === "percent"
              ? `${Math.round(control.valueAsNumber * 100)}%`
              : `${control.valueAsNumber.toFixed(2)}×`;
      }
    } catch (error) {
      onError(error);
    }
  };
  for (const control of inputs) {
    control.disabled = false;
    control.addEventListener("input", update, { signal: listeners.signal });
  }
  reset.disabled = false;
  reset.addEventListener(
    "click",
    () => {
      for (const control of inputs) control.value = control.defaultValue;
      update();
    },
    { signal: listeners.signal },
  );
  update();
  return () => {
    listeners.abort();
    for (const control of inputs) control.disabled = true;
    reset.disabled = true;
  };
}
