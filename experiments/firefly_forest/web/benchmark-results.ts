import { validLightSettings, type LightSettings } from "./light-controls";
import { validColorSettings, type ColorSettings } from "./color-controls";
import { validFogSettings, type FogSettings } from "./fog-controls";

export const BENCHMARK_VERSION = 20;
export const WARMUP_FRAMES = 120;
export const FIXED_DT = 1 / 60;
export const BASELINE_KEY = "firefly-forest.benchmark.baseline.v20";

export interface BenchmarkConfig {
  seed: number;
  frames: number;
  warmup: number;
  dt: number;
  count: number;
  movement: number;
  frequency: number;
  duration: number;
  flashRecovery: number;
  fireflyGlow: boolean;
  movementRandomness: number;
  blinkRandomness: number;
  size: number;
  fov: number;
  brushStrokes: boolean;
  fireflyRenderMode: "particles" | "trails" | "drag";
  camera: number[];
  cameraPath: "static" | "forest-loop-v1";
  sortCadence: "threshold-v1";
  colors: ColorSettings;
  lighting: LightSettings;
  fog: FogSettings;
  cssSize: number[];
  dpr: number;
  environment: string;
  scene: string;
  gpuTiming: boolean;
  browser: string;
}

export interface Statistics {
  mean: number;
  p95: number;
}

export interface BenchmarkResult {
  version: typeof BENCHMARK_VERSION;
  createdAt: string;
  config: BenchmarkConfig;
  viewCulling: boolean;
  elapsedMs: number;
  fps: number;
  frameMs: Statistics;
  gpuSamples: number;
  gpu: {
    total: Statistics;
    compute: Statistics;
    forest: Statistics;
    glow: Statistics;
  } | null;
}

export function statistics(samples: number[]): Statistics {
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    mean: samples.reduce((sum, value) => sum + value, 0) / samples.length,
    p95: sorted[Math.ceil(sorted.length * 0.95) - 1],
  };
}

const labels: Record<keyof BenchmarkConfig, string> = {
  seed: "seed",
  frames: "量測幀數",
  warmup: "預熱幀數",
  dt: "時間步長",
  count: "粒子數",
  movement: "移動速度",
  frequency: "閃爍頻率",
  duration: "閃爍時長",
  flashRecovery: "閃爍恢復",
  fireflyGlow: "粒子 Glow",
  movementRandomness: "移動隨機程度",
  blinkRandomness: "閃爍隨機程度",
  size: "螢火蟲大小",
  fov: "FOV",
  brushStrokes: "森林筆觸",
  fireflyRenderMode: "螢火蟲顯示模式",
  camera: "攝影機／輸出尺寸",
  cameraPath: "鏡頭路徑",
  sortCadence: "排序規則",
  colors: "調色設定",
  lighting: "局部光照設定",
  fog: "體積霧設定",
  cssSize: "視窗尺寸",
  dpr: "像素比例",
  environment: "GPU／輸出模式",
  scene: "森林模型",
  gpuTiming: "GPU 計時模式",
  browser: "瀏覽器",
};

export function differences(a: BenchmarkConfig, b: BenchmarkConfig): string[] {
  return (Object.keys(labels) as (keyof BenchmarkConfig)[])
    .filter((key) => JSON.stringify(a[key]) !== JSON.stringify(b[key]))
    .map((key) => labels[key]);
}

/** Treat localStorage as untrusted/versioned input, including partial older results. */
export function parseBaseline(raw: string): BenchmarkResult {
  const value = JSON.parse(raw) as BenchmarkResult;
  const finite = (number: unknown) => typeof number === "number" && Number.isFinite(number);
  const positive = (number: unknown) => finite(number) && (number as number) > 0;
  const stat = (s: Statistics | undefined) =>
    s && finite(s.mean) && s.mean >= 0 && finite(s.p95) && s.p95 >= 0;
  const c = value?.config;
  if (
    value?.version !== BENCHMARK_VERSION ||
    !c ||
    !Number.isInteger(c.seed) ||
    c.seed < 1 ||
    c.seed > 0xffffffff ||
    !Number.isInteger(c.frames) ||
    c.frames < 1 ||
    c.frames > 10_000 ||
    c.warmup !== WARMUP_FRAMES ||
    c.dt !== FIXED_DT ||
    !Number.isInteger(c.count) ||
    c.count < 10_000 ||
    c.count > 100_000 ||
    !finite(c.movement) ||
    c.movement < 0 ||
    c.movement > 1 ||
    !finite(c.frequency) ||
    c.frequency < 0 ||
    c.frequency > 1 ||
    !finite(c.duration) ||
    c.duration < 0.1 ||
    c.duration > 2 ||
    !finite(c.flashRecovery) ||
    c.flashRecovery < 0.1 ||
    c.flashRecovery > 5 ||
    !finite(c.movementRandomness) ||
    c.movementRandomness < 0 ||
    c.movementRandomness > 2 ||
    !finite(c.blinkRandomness) ||
    c.blinkRandomness < 0 ||
    c.blinkRandomness > 2 ||
    !finite(c.size) ||
    c.size < 0.25 ||
    c.size > 3 ||
    !finite(c.fov) ||
    c.fov < 20 ||
    c.fov > 100 ||
    typeof c.brushStrokes !== "boolean" ||
    typeof c.fireflyGlow !== "boolean" ||
    !["particles", "trails", "drag"].includes(c.fireflyRenderMode) ||
    !["static", "forest-loop-v1"].includes(c.cameraPath) ||
    c.sortCadence !== "threshold-v1" ||
    !validColorSettings(c.colors) ||
    !validLightSettings(c.lighting) ||
    !validFogSettings(c.fog) ||
    !Array.isArray(c.camera) ||
    c.camera.length !== 34 ||
    !c.camera.every(finite) ||
    !positive(c.camera[32]) ||
    !positive(c.camera[33]) ||
    !Array.isArray(c.cssSize) ||
    c.cssSize.length !== 2 ||
    !c.cssSize.every(positive) ||
    !positive(c.dpr) ||
    typeof c.environment !== "string" ||
    typeof c.scene !== "string" ||
    typeof c.browser !== "string" ||
    typeof c.gpuTiming !== "boolean" ||
    typeof value.viewCulling !== "boolean" ||
    typeof value.createdAt !== "string" ||
    !Number.isFinite(Date.parse(value.createdAt)) ||
    !positive(value.elapsedMs) ||
    !positive(value.fps) ||
    !stat(value.frameMs) ||
    (c.gpuTiming
      ? value.gpuSamples !== c.frames ||
        !value.gpu ||
        !stat(value.gpu.total) ||
        !stat(value.gpu.compute) ||
        !stat(value.gpu.forest) ||
        !stat(value.gpu.glow)
      : value.gpu !== null || value.gpuSamples !== 0)
  ) {
    throw new Error("儲存的基準格式不相容，請重新量測並儲存。");
  }
  return value;
}
