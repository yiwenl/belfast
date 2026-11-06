/** The camera follows the path in Rust; DOM input only supplies signed CSS pixels. */
export function bindWalkInput(
  canvas: HTMLCanvasElement,
  app: { scroll(delta: number): void },
): () => void {
  const listeners = new AbortController();
  canvas.addEventListener(
    "wheel",
    (event) => {
      if (event.ctrlKey) return;
      event.preventDefault();
      const scale = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? canvas.clientHeight : 1;
      app.scroll(event.deltaY * scale);
    },
    { signal: listeners.signal, passive: false },
  );
  return () => listeners.abort();
}

export function bindPointerInput(
  canvas: HTMLCanvasElement,
  app: { setPointer(x: number, y: number): void; clearPointer(): void },
) {
  const listeners = new AbortController();
  let pointer: { x: number; y: number } | undefined;
  const move = (event: PointerEvent) => {
    pointer = { x: event.clientX, y: event.clientY };
  };
  const clear = () => {
    pointer = undefined;
    app.clearPointer();
  };
  canvas.addEventListener("pointerenter", move, { signal: listeners.signal });
  canvas.addEventListener("pointermove", move, { signal: listeners.signal });
  canvas.addEventListener("pointerleave", clear, { signal: listeners.signal });
  canvas.addEventListener("pointercancel", clear, { signal: listeners.signal });
  window.addEventListener("blur", clear, { signal: listeners.signal });
  return {
    update() {
      if (!pointer) return;
      const rect = canvas.getBoundingClientRect();
      // Fullscreen object-fit: contain can letterbox inside the canvas element itself.
      const scale = Math.min(rect.width / canvas.width, rect.height / canvas.height);
      const width = canvas.width * scale;
      const height = canvas.height * scale;
      if (!(width > 0 && height > 0)) {
        app.clearPointer();
        return;
      }
      const x = (pointer.x - rect.left - (rect.width - width) * 0.5) / width;
      const y = (pointer.y - rect.top - (rect.height - height) * 0.5) / height;
      if (x < 0 || x > 1 || y < 0 || y > 1) {
        app.clearPointer();
        return;
      }
      app.setPointer(x * 2 - 1, 1 - y * 2);
    },
    dispose() {
      listeners.abort();
      clear();
    },
  };
}
