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
