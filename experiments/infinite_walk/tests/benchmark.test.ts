import { test } from "node:test";
import assert from "node:assert/strict";
import { createFrameBenchmark } from "../web/benchmark";

test("measures completed GPU frames over elapsed time after warmup", () => {
  const benchmark = createFrameBenchmark(100, 1000);
  benchmark.tick(0, 0, true);
  benchmark.tick(100, 10, true);
  for (let i = 1; i < 100; i++) benchmark.tick(100 + i * 10, 10 + i, true);
  const result = benchmark.tick(1100, 110, true)!;
  assert.ok(result, "the completed sample must return measurements");
  assert.equal(result.averageFps, 100);
  assert.equal(result.frames, 100);
  assert.equal(result.elapsedMs, 1000);
  assert.equal(result.p95CompletionBatchMs, 10);
  assert.equal(result.valid, true);
});

test("missed renders remain in elapsed time and frame latency", () => {
  const benchmark = createFrameBenchmark(0, 100);
  benchmark.tick(0, 0, true);
  benchmark.tick(25, 0, true);
  benchmark.tick(50, 1, true);
  benchmark.tick(75, 1, true);
  const result = benchmark.tick(100, 2, true)!;
  assert.ok(result, "the completed sample must include missed renders");
  assert.equal(result.averageFps, 20);
  assert.equal(result.p95CompletionBatchMs, 50);
  assert.equal(result.samplesWithoutCompletion, 2);
});

test("a hidden tab invalidates the run instead of discarding slow time", () => {
  const benchmark = createFrameBenchmark(0, 100);
  benchmark.tick(0, 0, true);
  const result = benchmark.tick(50, 1, false)!;
  assert.ok(result, "hiding the tab must terminate the run as invalid");
  assert.equal(result.valid, false);
  assert.match(result.reason!, /hidden/);
  assert.equal(benchmark.tick(100, 2, true), undefined);
});

test("counts batched completions and excludes submissions still waiting on the GPU", () => {
  const benchmark = createFrameBenchmark(0, 100);
  benchmark.tick(0, 20, true);
  benchmark.tick(25, 20, true);
  benchmark.tick(50, 23, true);
  const result = benchmark.tick(100, 25, true)!;
  assert.equal(result.frames, 5);
  assert.equal(result.averageFps, 50);
  assert.equal(result.rafAverageFps, 30);
});
