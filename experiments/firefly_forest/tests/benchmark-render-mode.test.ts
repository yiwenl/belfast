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
  const drag = { ...particles, fireflyRenderMode: "drag" } as BenchmarkConfig;
  assert.equal(differences(particles, drag).length, 1);
  assert.equal(differences(trails, drag).length, 1);
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
    flashRecovery: 0.65,
    fireflyGlow: true,
  });
  baseline.config.fog.noiseFrequency = 1;
  for (const mode of ["particles", "trails", "drag"]) {
    baseline.config.fireflyRenderMode = mode;
    assert.equal(parseBaseline(JSON.stringify(baseline)).config.fireflyRenderMode, mode);
  }
  for (const mode of [undefined, "unknown", true]) {
    baseline.config.fireflyRenderMode = mode;
    assert.throws(() => parseBaseline(JSON.stringify(baseline)));
  }
  baseline.config.fireflyRenderMode = "drag";
  const otherRecovery = { ...baseline.config, flashRecovery: 2 };
  assert.deepEqual(differences(baseline.config, otherRecovery), ["閃爍恢復"]);
  for (const recovery of [0.1, 0.65, 5]) {
    baseline.config.flashRecovery = recovery;
    assert.equal(parseBaseline(JSON.stringify(baseline)).config.flashRecovery, recovery);
  }
  for (const recovery of [undefined, null, "1", -1, 0, 5.1]) {
    baseline.config.flashRecovery = recovery;
    assert.throws(() => parseBaseline(JSON.stringify(baseline)));
  }
  baseline.config.flashRecovery = 0.65;
  assert.deepEqual(differences(baseline.config, { ...baseline.config, fireflyGlow: false }), [
    "粒子 Glow",
  ]);
  for (const glow of [true, false]) {
    baseline.config.fireflyGlow = glow;
    assert.equal(parseBaseline(JSON.stringify(baseline)).config.fireflyGlow, glow);
  }
  for (const glow of [undefined, null, "false", 0, 1]) {
    baseline.config.fireflyGlow = glow;
    assert.throws(() => parseBaseline(JSON.stringify(baseline)));
  }
});
