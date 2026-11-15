export interface OrbitTarget {
  pointerDown(x: number, y: number, button: number, pan: boolean): void;
  pointerMove(x: number, y: number, width: number, height: number): void;
  pointerUp(): void;
  scroll(delta: number): void;
}

/** Forward browser events only. Orbit, pan, zoom and damping are calculated in Rust. */
export function bindOrbitInput(canvas: HTMLCanvasElement, app: OrbitTarget): () => void {
  const listeners = new AbortController();
  const options = { signal: listeners.signal };
  let pointerId: number | null = null;
  let buttonMask = 0;

  function finishDrag() {
    if (pointerId === null) return;
    const previous = pointerId;
    pointerId = null;
    app.pointerUp();
    if (canvas.hasPointerCapture(previous)) canvas.releasePointerCapture(previous);
  }

  canvas.addEventListener(
    "pointerdown",
    (event) => {
      if (pointerId !== null || (event.button !== 0 && event.button !== 1)) return;
      event.preventDefault();
      pointerId = event.pointerId;
      buttonMask = event.button === 0 ? 1 : 4;
      canvas.setPointerCapture(pointerId);
      const rect = canvas.getBoundingClientRect();
      app.pointerDown(
        event.clientX - rect.left,
        event.clientY - rect.top,
        event.button,
        event.shiftKey,
      );
    },
    options,
  );

  canvas.addEventListener(
    "pointermove",
    (event) => {
      if (event.pointerId !== pointerId) return;
      if (!(event.buttons & buttonMask)) {
        finishDrag();
        return;
      }
      const rect = canvas.getBoundingClientRect();
      app.pointerMove(event.clientX - rect.left, event.clientY - rect.top, rect.width, rect.height);
    },
    options,
  );

  for (const type of ["pointerup", "pointercancel", "lostpointercapture"] as const) {
    canvas.addEventListener(
      type,
      (event) => {
        if (event.pointerId === pointerId) finishDrag();
      },
      options,
    );
  }
  window.addEventListener("blur", finishDrag, options);
  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const scale = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? canvas.clientHeight : 1;
      app.scroll(event.deltaY * scale);
    },
    { ...options, passive: false },
  );

  return () => {
    finishDrag();
    listeners.abort();
  };
}
