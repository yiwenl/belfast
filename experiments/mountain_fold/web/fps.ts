/** Average animation-frame rate, refreshed twice per second. */
export function createFpsMeter(element: HTMLElement) {
  let sampleStart: number | undefined;
  let frames = 0;

  function reset() {
    sampleStart = undefined;
    frames = 0;
    element.textContent = "— FPS";
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
        element.textContent = `${Math.round((frames * 1000) / elapsed)} FPS`;
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
