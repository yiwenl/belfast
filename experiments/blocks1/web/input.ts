import type { ExperimentApp } from "../pkg/experiment";

export function bindFluidInput(canvas: HTMLCanvasElement, app: ExperimentApp): () => void {
  const listeners = new AbortController();
  const options = { signal: listeners.signal };
  const clear = () => app.clearPointer();
  const move = (event: PointerEvent) => {
    if (!event.isPrimary) return;
    const rect = canvas.getBoundingClientRect();
    app.pointerMove(event.clientX - rect.left, event.clientY - rect.top, rect.width, rect.height);
  };
  canvas.addEventListener("pointerenter", move, options);
  canvas.addEventListener("pointermove", move, options);
  canvas.addEventListener("pointerleave", clear, options);
  canvas.addEventListener("pointercancel", clear, options);
  canvas.addEventListener(
    "pointerup",
    (event) => {
      if (event.pointerType !== "mouse") clear();
    },
    options,
  );
  window.addEventListener("blur", clear, options);
  window.addEventListener("resize", clear, options);
  window.addEventListener("pagehide", clear, options);
  document.addEventListener("visibilitychange", clear, options);
  return () => {
    listeners.abort();
    clear();
  };
}
