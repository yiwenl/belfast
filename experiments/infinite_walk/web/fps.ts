/** Average animation-frame rate, refreshed twice per second. */
export function createFpsMeter(update: (fps: number | null) => void) {
  let sampleStart: number | undefined;
  let frames = 0;

  function reset() {
    sampleStart = undefined;
    frames = 0;
    update(null);
  }

  document.addEventListener("visibilitychange", reset);

  return {
    tick(timestampMs: number) {
      if (document.hidden) return;
      if (sampleStart === undefined) {
        sampleStart = timestampMs;
        return;
      }
      frames += 1;
      const elapsed = timestampMs - sampleStart;
      if (elapsed >= 500) {
        update(Math.round((frames * 1000) / elapsed));
        sampleStart = timestampMs;
        frames = 0;
      }
    },
    dispose() {
      document.removeEventListener("visibilitychange", reset);
      reset();
    },
  };
}
