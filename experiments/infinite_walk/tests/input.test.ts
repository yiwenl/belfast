import assert from "node:assert/strict";
import test from "node:test";
import { bindWalkInput } from "../web/input.ts";

test("normalizes forward/backward wheel units, preserves pinch zoom and detaches", () => {
  const canvas = Object.assign(new EventTarget(), { clientHeight: 400 });
  const distances: number[] = [];
  const detach = bindWalkInput(canvas as unknown as HTMLCanvasElement, {
    scroll: (delta) => distances.push(delta),
  });
  const wheel = (deltaY: number, deltaMode: number, ctrlKey = false) => {
    const event = Object.assign(new Event("wheel", { cancelable: true }), {
      deltaY,
      deltaMode,
      ctrlKey,
    });
    canvas.dispatchEvent(event);
    return event.defaultPrevented;
  };
  assert.equal(wheel(100, 0), true);
  wheel(-2, 1);
  wheel(1, 2);
  assert.deepEqual(distances, [100, -32, 400]);
  assert.equal(wheel(-50, 0, true), false);
  assert.equal(distances.length, 3);
  detach();
  assert.equal(wheel(20, 0), false);
  assert.equal(distances.length, 3);
});
