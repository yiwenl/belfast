import { WheelStepper } from "./wheel";

export interface WalkTarget {
  stepCamera(direction: number): void;
  lookAround(yawDelta: number, pitchDelta: number): void;
}

/** Browser input only; camera positions, orientations and transitions stay in Rust. */
export function bindWalkInput(
  canvas: HTMLCanvasElement,
  app: WalkTarget,
  wheelTarget: HTMLElement = canvas,
): () => void {
  const listeners = new AbortController();
  const wheel = new WheelStepper();
  let drag: { id: number; x: number; y: number } | undefined;
  function endDrag() {
    if (!drag) return;
    const id = drag.id;
    drag = undefined;
    if (canvas.hasPointerCapture(id)) canvas.releasePointerCapture(id);
  }
  canvas.addEventListener(
    "pointerdown",
    (event) => {
      if (!event.isPrimary || event.button !== 0 || drag) return;
      event.preventDefault();
      canvas.focus({ preventScroll: true });
      canvas.setPointerCapture(event.pointerId);
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
    },
    { signal: listeners.signal },
  );
  canvas.addEventListener(
    "pointermove",
    (event) => {
      if (!drag || event.pointerId !== drag.id) return;
      if (event.pointerType === "mouse" && (event.buttons & 1) === 0) {
        endDrag();
        return;
      }
      event.preventDefault();
      const radiansPerPixel = (2 * Math.PI) / Math.max(1, canvas.clientHeight);
      app.lookAround(
        -(event.clientX - drag.x) * radiansPerPixel,
        -(event.clientY - drag.y) * radiansPerPixel,
      );
      drag.x = event.clientX;
      drag.y = event.clientY;
    },
    { signal: listeners.signal },
  );
  for (const type of ["pointerup", "pointercancel", "lostpointercapture"] as const) {
    canvas.addEventListener(
      type,
      (event) => {
        if (event.pointerId === drag?.id) endDrag();
      },
      { signal: listeners.signal },
    );
  }
  window.addEventListener("blur", endDrag, { signal: listeners.signal });
  wheelTarget.addEventListener(
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
  return () => {
    endDrag();
    listeners.abort();
  };
}
