import type { ExperimentApp } from "../pkg/experiment";
import { colorSettings } from "./color-controls";
import { lightSettings } from "./light-controls";
import { fogSettings } from "./fog-controls";
import {
  BASELINE_KEY,
  BENCHMARK_VERSION,
  FIXED_DT,
  WARMUP_FRAMES,
  differences,
  parseBaseline,
  statistics,
  type BenchmarkConfig,
  type BenchmarkResult,
} from "./benchmark-results";

type Phase = "settling" | "warming" | "resetting" | "measuring" | "draining";
interface Run {
  phase: Phase;
  phaseStart: number;
  config: BenchmarkConfig;
  viewCulling: boolean;
  warmup: number;
  intervals: number[];
  lastTimestamp: number | null;
  gpu: number[] | null;
  cameraSamples: number[][];
}

export function createBenchmark(
  app: ExperimentApp,
  canvas: HTMLCanvasElement,
  lockScene: (locked: boolean) => void,
) {
  const get = <T extends HTMLElement>(id: string) => document.querySelector<T>(`#${id}`)!;
  const seed = get<HTMLInputElement>("benchmark-seed");
  const frames = get<HTMLInputElement>("benchmark-frames");
  const moving = get<HTMLInputElement>("benchmark-moving");
  const culling = get<HTMLInputElement>("view-culling");
  const start = get<HTMLButtonElement>("benchmark-start");
  const cancel = get<HTMLButtonElement>("benchmark-cancel");
  const save = get<HTMLButtonElement>("benchmark-save");
  const clear = get<HTMLButtonElement>("benchmark-clear");
  const download = get<HTMLAnchorElement>("benchmark-download");
  const status = get<HTMLElement>("benchmark-status");
  const comparison = get<HTMLElement>("benchmark-comparison");
  const summary = get<HTMLElement>("benchmark-result-summary");
  const baselineSummary = get<HTMLElement>("benchmark-baseline-summary");
  const rows = get<HTMLTableSectionElement>("benchmark-values");
  const listeners = new AbortController();
  const options = { signal: listeners.signal };
  let ready = false;
  let run: Run | null = null;
  let latest: BenchmarkResult | null = null;
  let baseline: BenchmarkResult | null = null;

  const gpuSupported = app.gpuTimingSupported();
  get("benchmark-gpu-support").textContent = gpuSupported
    ? "GPU timestamp 可用 · 整輪結束後讀回"
    : "此瀏覽器／GPU 不支援 timestamp；仍可量測 FPS";

  const number = (id: string) => get<HTMLInputElement>(id).valueAsNumber;
  function config(): BenchmarkConfig {
    return {
      seed: seed.valueAsNumber,
      frames: frames.valueAsNumber,
      warmup: WARMUP_FRAMES,
      dt: FIXED_DT,
      count: number("particle-count"),
      movement: number("movement-speed"),
      frequency: number("blink-frequency"),
      duration: number("flash-duration"),
      movementRandomness: number("movement-randomness"),
      blinkRandomness: number("blink-randomness"),
      size: number("firefly-size"),
      fov: number("camera-fov"),
      brushStrokes: get<HTMLInputElement>("brush-enabled").checked,
      camera: Array.from(app.benchmarkCamera()),
      cameraPath: moving.checked ? "forest-loop-v1" : "static",
      sortCadence: "threshold-v1",
      colors: colorSettings(),
      lighting: lightSettings(),
      fog: fogSettings(),
      cssSize: [canvas.clientWidth, canvas.clientHeight],
      dpr: window.devicePixelRatio,
      environment: app.benchmarkEnvironment(),
      scene: "forest.ply:4116349",
      gpuTiming: gpuSupported,
      browser: navigator.userAgent,
    };
  }

  function buttons() {
    const busy = run !== null;
    start.disabled = busy || !ready;
    cancel.disabled = !busy;
    save.disabled = busy || latest === null;
    clear.disabled = busy || baseline === null;
    seed.disabled = busy;
    frames.disabled = busy;
    culling.disabled = busy;
    moving.disabled = busy;
  }

  function describe(result: BenchmarkResult) {
    const c = result.config;
    return `${c.cameraPath === "static" ? "固定視角" : "移動路徑 v1"} · ${result.viewCulling ? "簡化開" : "簡化關"} · ${c.count.toLocaleString()} 隻 · seed ${c.seed} · ${c.frames} 幀`;
  }

  function showResults() {
    const mismatches = latest && baseline ? differences(latest.config, baseline.config) : [];
    const comparable = latest !== null && baseline !== null && mismatches.length === 0;
    comparison.textContent = mismatches.length
      ? `條件不同：${mismatches.join("、")}。不計算提升百分比。`
      : comparable
        ? "條件相同，可比較。FPS 越高越好；耗時越低越好。"
        : "儲存一組基準後，切換「視角外簡化」再量測。";
    summary.textContent = latest ? `本次：${describe(latest)}` : "本次：尚未量測";
    baselineSummary.textContent = baseline
      ? `基準：${describe(baseline)} · ${new Date(baseline.createdAt).toLocaleString()}`
      : "基準：尚未儲存";
    const metrics: [string, (r: BenchmarkResult) => number | undefined, boolean][] = [
      ["平均 FPS", (r) => r.fps, true],
      ["幀時間平均 ms", (r) => r.frameMs.mean, false],
      ["幀時間 P95 ms", (r) => r.frameMs.p95, false],
      ["GPU 整幀平均 ms", (r) => r.gpu?.total.mean, false],
      ["GPU 整幀 P95 ms", (r) => r.gpu?.total.p95, false],
      ["螢火蟲 compute ms", (r) => r.gpu?.compute.mean, false],
      ["森林＋光照＋霧 ms", (r) => r.gpu?.forest.mean, false],
      ["螢火蟲繪製 ms", (r) => r.gpu?.glow.mean, false],
    ];
    rows.replaceChildren(
      ...metrics.map(([label, read, higherIsBetter]) => {
        const row = document.createElement("tr");
        const heading = document.createElement("th");
        heading.scope = "row";
        heading.textContent = label;
        row.append(heading);
        const current = latest ? read(latest) : undefined;
        const previous = baseline ? read(baseline) : undefined;
        for (const value of [current, previous]) {
          const cell = document.createElement("td");
          cell.textContent = value === undefined ? "—" : value.toFixed(2);
          row.append(cell);
        }
        const delta = document.createElement("td");
        if (comparable && current !== undefined && previous !== undefined && previous > 0) {
          const change = (current / previous - 1) * 100;
          delta.textContent = `${change > 0 ? "+" : ""}${change.toFixed(1)}%`;
          if (Math.abs(change) >= 0.05) {
            delta.className = (higherIsBetter ? change > 0 : change < 0) ? "better" : "worse";
          }
        } else {
          delta.textContent = "—";
        }
        row.append(delta);
        return row;
      }),
    );
    buttons();
  }

  try {
    const stored = localStorage.getItem(BASELINE_KEY);
    if (stored) baseline = parseBaseline(stored);
  } catch (error) {
    status.textContent = `無法讀取基準：${String(error)}`;
  }

  function abort(message: string) {
    if (!run) return;
    run = null;
    app.endBenchmark();
    lockScene(false);
    status.textContent = message;
    buttons();
  }

  function finish() {
    app.checkGpuError();
    const completed = run!;
    const elapsedMs = completed.intervals.reduce((sum, value) => sum + value, 0);
    const samples = completed.gpu;
    const gpuStats = (component: number) =>
      statistics(samples!.filter((_, index) => index % 4 === component));
    latest = {
      version: BENCHMARK_VERSION,
      createdAt: new Date().toISOString(),
      config: completed.config,
      viewCulling: completed.viewCulling,
      elapsedMs,
      fps: (completed.intervals.length * 1000) / elapsedMs,
      frameMs: statistics(completed.intervals),
      gpuSamples: samples ? samples.length / 4 : 0,
      gpu: samples
        ? {
            total: gpuStats(0),
            compute: gpuStats(1),
            forest: gpuStats(2),
            glow: gpuStats(3),
          }
        : null,
    };
    // Export after timing has ended, so serialization cannot affect samples.
    const report = {
      ...latest,
      frameIntervalsMs: completed.intervals,
      gpuFrameMs: samples,
      gpuFrameLayout: ["total", "compute", "forest", "glow"],
      cameraSamples: completed.cameraSamples,
      mergedOcclusion: app.mergedOcclusion(),
      cameraSampleLayout:
        "frame index, view[16], projection[16], width, height; every 60 frames and final",
    };
    download.href = `data:application/json;charset=utf-8,${encodeURIComponent(JSON.stringify(report))}`;
    download.download = `firefly-benchmark-${latest.createdAt.replaceAll(":", "-")}.json`;
    download.hidden = false;
    run = null;
    app.endBenchmark();
    lockScene(false);
    status.textContent = `完成 ${latest.config.frames} 幀 · ${(elapsedMs / 1000).toFixed(2)} 秒${samples ? ` · GPU ${latest.gpuSamples} 筆` : " · GPU 耗時不可用"}`;
    showResults();
  }

  start.addEventListener(
    "click",
    () => {
      if (!ready || run || !seed.reportValidity() || !frames.reportValidity()) return;
      if (document.hidden || !canvas.clientWidth || !canvas.clientHeight) return;
      try {
        lockScene(true);
        app.beginBenchmark(seed.valueAsNumber, moving.checked);
        run = {
          phase: "settling",
          phaseStart: performance.now(),
          config: config(),
          viewCulling: culling.checked,
          warmup: 0,
          intervals: [],
          lastTimestamp: null,
          gpu: null,
          cameraSamples: [],
        };
        status.textContent = "準備起始視角，等待森林排序…";
        buttons();
      } catch (error) {
        app.endBenchmark();
        run = null;
        lockScene(false);
        status.textContent = `無法開始：${String(error)}`;
        buttons();
      }
    },
    options,
  );
  cancel.addEventListener("click", () => abort("已取消；這輪結果不會儲存。"), options);
  culling.addEventListener("change", () => app.setViewCulling(culling.checked), options);
  save.addEventListener(
    "click",
    () => {
      if (!latest || run) return;
      try {
        localStorage.setItem(BASELINE_KEY, JSON.stringify(latest));
        baseline = latest;
        status.textContent = "基準已儲存到此瀏覽器，重新整理後仍保留。";
        showResults();
      } catch (error) {
        status.textContent = `無法儲存基準：${String(error)}`;
      }
    },
    options,
  );
  clear.addEventListener(
    "click",
    () => {
      try {
        localStorage.removeItem(BASELINE_KEY);
        baseline = null;
        showResults();
      } catch (error) {
        status.textContent = `無法清除基準：${String(error)}`;
      }
    },
    options,
  );
  document.addEventListener(
    "visibilitychange",
    () => {
      if (document.hidden) abort("分頁已隱藏，量測取消，請保持頁面可見再開始。");
    },
    options,
  );
  window.addEventListener("resize", () => abort("視窗尺寸改變，量測取消。"), options);
  app.setViewCulling(culling.checked);
  showResults();

  return {
    setReady() {
      if (!ready) status.textContent = "就緒。可先儲存基準，再切換簡化開關比較。";
      ready = true;
      buttons();
    },
    /** Returns true while it owns rendering, including setup/readback RAFs. */
    tick(timestampMs: number): boolean {
      const current = run;
      if (!current) return false;
      try {
        // GPU errors are asynchronous, including after the final submitted
        // frame. Keep checking while uploads/results are draining.
        app.checkGpuError();
        if (
          document.hidden ||
          canvas.clientWidth !== current.config.cssSize[0] ||
          canvas.clientHeight !== current.config.cssSize[1] ||
          window.devicePixelRatio !== current.config.dpr
        ) {
          abort("顯示條件改變，量測取消。");
          return true;
        }
        if (
          current.phase !== "measuring" &&
          current.phase !== "warming" &&
          performance.now() - current.phaseStart > 15_000
        ) {
          abort("等待排序或 GPU 逾時，這輪結果無效。");
          return true;
        }
        if (current.phase === "draining") {
          if (gpuSupported && !current.gpu) {
            const data = app.takeGpuTimings();
            if (data.length) {
              if (
                data.length !== current.config.frames * 4 ||
                !Array.from(data).every((value) => Number.isFinite(value) && value >= 0)
              ) {
                throw new Error("GPU 計時資料不完整");
              }
              current.gpu = Array.from(data);
            }
          }
          if (app.gpuIdle() && (!gpuSupported || current.gpu)) finish();
          return true;
        }
        if (current.phase === "resetting") {
          if (!app.gpuIdle()) return true;
          current.phase = "measuring";
          current.phaseStart = performance.now();
          status.textContent = `量測中 0 / ${current.config.frames}`;
        }
        if (current.phase === "measuring") {
          // Each interval ends on the RAF after its submitted frame. This
          // measures exactly N intervals, without submitting an extra frame.
          if (current.lastTimestamp !== null) {
            current.intervals.push(timestampMs - current.lastTimestamp);
          }
          if (current.intervals.length === current.config.frames) {
            current.phase = "draining";
            current.phaseStart = performance.now();
            status.textContent = "量測完成，讀取 GPU 結果…";
            app.finishGpuTiming();
            return true;
          }
          if (!app.benchmarkFrame()) throw new Error("繪圖表面暫時不可用");
          const frame = current.intervals.length;
          if (frame % 60 === 0 || frame === current.config.frames - 1) {
            current.cameraSamples.push([frame, ...app.benchmarkCamera()]);
          }
          current.lastTimestamp = timestampMs;
          if (current.intervals.length % 30 === 0) {
            status.textContent = `量測中 ${current.intervals.length} / ${current.config.frames}`;
          }
          return true;
        }
        if (!app.benchmarkFrame()) throw new Error("繪圖表面暫時不可用");
        if (current.phase === "settling" && app.benchmarkReady()) {
          current.phase = "warming";
          status.textContent = `預熱 0 / ${WARMUP_FRAMES}`;
        } else if (current.phase === "warming") {
          current.warmup += 1;
          if (current.warmup % 30 === 0) {
            status.textContent = `預熱 ${current.warmup} / ${WARMUP_FRAMES}`;
          }
          if (current.warmup === WARMUP_FRAMES) {
            app.resetBenchmark(current.config.seed, current.config.frames);
            current.phase = "resetting";
            current.phaseStart = performance.now();
            status.textContent = "重設相同 seed，等待 GPU 準備完成…";
          }
        }
      } catch (error) {
        abort(`量測取消：${String(error)}`);
      }
      return true;
    },
    dispose() {
      abort("量測已停止。");
      listeners.abort();
      start.disabled = true;
    },
  };
}
