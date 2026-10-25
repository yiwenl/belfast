import { WheelStepper } from "./wheel";

export interface WalkTarget {
  stepCamera(direction: number): void;
}

/** Browser input only; camera positions, orientations and transitions stay in Rust. */
export function bindWalkInput(canvas: HTMLCanvasElement, app: WalkTarget): () => void {
  const listeners = new AbortController();
  const wheel = new WheelStepper();
  canvas.addEventListener(
    "wheel",
    (event) => {
      if (event.ctrlKey || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return;
      event.preventDefault();
      const direction = wheel.push(event.deltaY, event.deltaMode, canvas.clientHeight);
      if (direction) app.stepCamera(direction);
    },
    { passive: false, signal: listeners.signal },
  );
  // Arrow keys are a discrete, accessible alternative to scrolling.
  window.addEventListener(
    "keydown",
    (event) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, select, textarea, button, [contenteditable]")) return;
      if (event.key === "ArrowDown" || event.key === "ArrowRight") {
        event.preventDefault();
        app.stepCamera(1);
      } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
        event.preventDefault();
        app.stepCamera(-1);
      }
    },
    { signal: listeners.signal },
  );
  return () => listeners.abort();
}
