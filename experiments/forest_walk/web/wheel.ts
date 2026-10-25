/** One wheel notch selects one pose; small trackpad deltas accumulate. */
export class WheelStepper {
  private accumulated = 0;

  push(delta: number, mode: number, height: number, ctrlKey = false): -1 | 0 | 1 {
    if (ctrlKey || !Number.isFinite(delta) || delta === 0) return 0;
    const pixels = delta * (mode === 1 ? 16 : mode === 2 ? height : 1);
    if (!Number.isFinite(pixels)) return 0;
    if (Math.sign(pixels) !== Math.sign(this.accumulated)) this.accumulated = 0;
    this.accumulated += pixels;
    if (Math.abs(this.accumulated) < 80) return 0;
    this.accumulated = 0;
    return pixels > 0 ? 1 : -1;
  }
}
