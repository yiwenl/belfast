export interface FrameMeasurement {
  valid: boolean;
  reason?: string;
  frames: number;
  samplesWithoutCompletion: number;
  elapsedMs: number;
  averageFps: number;
  rafAverageFps: number;
  p95CompletionBatchMs: number;
}

/** Counts GPU completion callbacks, including batches; never trims stalls from a sample. */
export function createFrameBenchmark(warmupMs: number, durationMs: number) {
  let began: number | undefined;
  let start: number | undefined;
  let previousRender = 0;
  let samplesWithoutCompletion = 0;
  let previousCount = 0,
    frames = 0;
  let rafFrames = 0;
  let done = false;
  const intervals: number[] = [];
  return {
    tick(now: number, completedCount: number, visible: boolean): FrameMeasurement | undefined {
      if (done) return;
      began ??= now;
      if (!visible) {
        done = true;
        return {
          valid: false,
          reason: "Page became hidden",
          frames: 0,
          samplesWithoutCompletion,
          elapsedMs: 0,
          averageFps: 0,
          rafAverageFps: 0,
          p95CompletionBatchMs: 0,
        };
      }
      if (now - began < warmupMs) return;
      if (start === undefined) {
        start = previousRender = now;
        previousCount = completedCount;
        return;
      }
      const completed = completedCount - previousCount;
      rafFrames++;
      previousCount = completedCount;
      frames += completed;
      if (completed > 0) {
        intervals.push(now - previousRender);
        previousRender = now;
      } else samplesWithoutCompletion++;
      const elapsedMs = now - start;
      if (elapsedMs < durationMs) return;
      done = true;
      intervals.sort((a, b) => a - b);
      return {
        valid: true,
        frames,
        samplesWithoutCompletion,
        elapsedMs,
        averageFps: (frames * 1000) / elapsedMs,
        rafAverageFps: (rafFrames * 1000) / elapsedMs,
        p95CompletionBatchMs: intervals[Math.max(0, Math.ceil(intervals.length * 0.95) - 1)] ?? 0,
      };
    },
  };
}
