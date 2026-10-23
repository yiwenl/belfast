import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  BENCHMARK_VERSION,
  differences,
  parseBaseline,
  type BenchmarkConfig,
} from "../web/benchmark-results";

test("particle and trail measurements cannot share a comparable baseline", () => {
  const particles = { fireflyRenderMode: "particles" } as BenchmarkConfig;
  const trails = { ...particles, fireflyRenderMode: "trails" } as BenchmarkConfig;
  assert.equal(differences(particles, trails).length, 1);
});

test("baseline loading validates the firefly render mode", () => {
  const baseline = JSON.parse(
    readFileSync(
      new URL("../benchmarks/2026-10-20-volumetric-fog/off.json", import.meta.url),
      "utf8",
    ),
  );
  baseline.version = BENCHMARK_VERSION;
  // Bring the archived v6 measurement up to the current control schema.
  Object.assign(baseline.config, {
    movementRandomness: 0.6,
    blinkRandomness: 0.6,
    size: 0.75,
    brushStrokes: true,
  });
  baseline.config.fog.noiseFrequency = 1;
  for (const mode of ["particles", "trails"]) {
    baseline.config.fireflyRenderMode = mode;
    assert.equal(parseBaseline(JSON.stringify(baseline)).config.fireflyRenderMode, mode);
  }
  for (const mode of [undefined, "unknown", true]) {
    baseline.config.fireflyRenderMode = mode;
    assert.throws(() => parseBaseline(JSON.stringify(baseline)));
  }
});
