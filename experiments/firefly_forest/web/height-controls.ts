import type { ExperimentApp } from "../pkg/experiment";

export interface HeightSettings {
  base: number;
  spread: number;
  outlier: number;
  exponent: number;
  limit: number;
  noiseScale: number;
  noiseAmplitude: number;
}

export function heightSettings(): HeightSettings {
  const number = (id: string) => (document.getElementById(id) as HTMLInputElement).valueAsNumber;
  return {
    base: number("height-base"),
    spread: number("height-spread"),
    outlier: number("height-outlier"),
    exponent: number("height-exponent"),
    limit: number("height-limit"),
    noiseScale: number("height-noise-scale"),
    noiseAmplitude: number("height-noise-amplitude"),
  };
}

export function validHeightSettings(value: unknown): value is HeightSettings {
  if (!value || typeof value !== "object") return false;
  const c = value as HeightSettings;
  return [
    [c.base, 0.25, 3],
    [c.spread, 0, 1.5],
    [c.outlier, 0, 3],
    [c.exponent, 1, 16],
    [c.limit, 0.25, 3],
    [c.noiseScale, 0.5, 20],
    [c.noiseAmplitude, 0, 1.5],
  ].every(([n, min, max]) => Number.isFinite(n) && n >= min && n <= max);
}

export function bindHeightControls(app: ExperimentApp, onError: (error: unknown) => void) {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>(".firefly-height input"));
  const listeners = new AbortController();
  const update = () => {
    try {
      const c = heightSettings();
      app.setFireflyHeight(
        c.base,
        c.spread,
        c.outlier,
        c.exponent,
        c.limit,
        c.noiseScale,
        c.noiseAmplitude,
      );
      for (const input of inputs) {
        (document.getElementById(`${input.id}-value`) as HTMLOutputElement).value =
          input.valueAsNumber.toFixed(input.id === "height-exponent" ? 0 : 2);
      }
    } catch (error) {
      onError(error);
    }
  };
  for (const input of inputs) {
    input.disabled = false;
    input.addEventListener("input", update, { signal: listeners.signal });
  }
  update();
  return () => {
    listeners.abort();
    for (const input of inputs) input.disabled = true;
  };
}
