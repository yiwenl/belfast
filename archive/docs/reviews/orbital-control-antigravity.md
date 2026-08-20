# Antigravity review: Orbital control and EaseNumber

**Slug:** `orbital-control`  
**Reviewed:** 2026-05-24T17:20:00Z  
**Manifest:** `docs/reviews/queue/orbital-control.json`

## Summary

This is an exceptionally solid, clean, and robust implementation of orbital controls and smoothed animation variables (`EaseNumber`). The inclusion of a formal `destroy()` lifecycle method for DOM event cleanup and `Scheduler` unsubscribe tokens is a massive improvement over traditional Alfrid-style orbital controls, completely preventing resource leaks in modern SPAs and hot-reloading environments. The integration of `scheduling` in Vite and the type-safety of the entire codebase are excellent.

## Critical

None.

## Suggestions

- **Robust/Smooth Trackpad Zooming**:
  Currently, `wheelDelta()` divides `event.deltaY` by a static `120`:

  ```ts
  function wheelDelta(event: WheelEvent): number {
    return -event.deltaY / 120;
  }
  ```

  While perfect for standard mouse wheels (which click in increments of 120), macOS trackpads or high-precision scroll wheels yield much smaller, high-frequency delta values (e.g., `deltaY: 1` or `2` per event). This can make trackpad zooming extremely slow or jittery.
  _Recommendation:_ Consider incorporating a normalized wheel delta check that checks `event.deltaMode` or clamp the delta, or expose a `zoomSpeed` option in `OrbitalControlOptions` to give developers control over zoom sensitivity.

- **Provide a Pivot / Center Offset control**:
  In full-featured graphics applications, panning the camera (usually middle-mouse drag or Shift+drag) shifts the `center` pivot of the orbit.
  _Recommendation:_ In a future follow-up, you can easily add pan support to `OrbitalControl` by listening to middle-mouse drag and shifting the `center` coordinate along the camera's local right and up axes.

## Nits

- **`EaseNumber._checkLimit()` during `setTo`**:
  In `EaseNumber.setTo(value)`, the value is set directly:

  ```ts
  setTo(value: number): void {
    this._targetValue = this._value = value;
  }
  ```

  However, it does not call `_checkLimit()` immediately. If the user calls `setTo()` with a value exceeding limits defined later (or defined before), the value remains out of bounds until the next automatic frame `_update()` tick clamps it. Consider calling `this._checkLimit()` at the end of `setTo()`.

- **Explicit Type on `position` and `positionOffset`**:
  In `OrbitalControl.ts` (lines 36-37), `position` is typed as `[number, number, number]` while `positionOffset` is typed as `[number, number, number]`. You already defined `Vec3` in `math/types.ts`. Consider using `Vec3` for consistency, though since `Vec3` is `readonly [number, number, number]`, you'd need a mutable equivalent (e.g., `type MutVec3 = [number, number, number]`) to support writing to coordinate indices.

## Test plan gaps

None. The `camera-orbit` example compiles successfully, handles setup and cleanup beautifully, and serves as a stellar demonstration of the feature in action.
