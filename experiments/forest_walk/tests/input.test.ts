import assert from "node:assert/strict";
import test from "node:test";
import { bindWalkInput } from "../web/input.ts";

test("wheel changes one selected camera; pinch, horizontal scroll and teardown do not", () => {
  const prior = globalThis.window;
  const windowEvents = new EventTarget();
  Object.defineProperty(globalThis, "window", { configurable: true, value: windowEvents });
  const canvas = Object.assign(new EventTarget(), { clientHeight: 800 });
  let selected = 0;
  const dispose = bindWalkInput(canvas as unknown as HTMLCanvasElement, {
    stepCamera(direction) {
      selected += direction;
    },
  });
  const send = (deltaY: number, extra = {}) =>
    canvas.dispatchEvent(
      Object.assign(new Event("wheel", { cancelable: true }), {
        deltaY,
        deltaX: 0,
        deltaMode: 0,
        ctrlKey: false,
        ...extra,
      }),
    );
  try {
    send(120);
    assert.equal(selected, 1);
    send(-120);
    assert.equal(selected, 0);
    send(120, { ctrlKey: true });
    send(20, { deltaX: 80 });
    assert.equal(selected, 0);
    dispose();
    send(120);
    assert.equal(selected, 0);
  } finally {
    dispose();
    Object.defineProperty(globalThis, "window", { configurable: true, value: prior });
  }
});
