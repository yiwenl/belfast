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
    lookAround() {
      throw new Error("Scrolling must not rotate the camera");
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

test("primary drag rotates the view; other pointers, release and teardown cannot keep dragging", () => {
  const prior = globalThis.window;
  Object.defineProperty(globalThis, "window", { configurable: true, value: new EventTarget() });
  let captured: number | undefined;
  const canvas = Object.assign(new EventTarget(), {
    clientHeight: 800,
    focus() {},
    setPointerCapture(id: number) {
      captured = id;
    },
    hasPointerCapture(id: number) {
      return captured === id;
    },
    releasePointerCapture(id: number) {
      if (captured === id) captured = undefined;
    },
  });
  const turns: number[][] = [];
  let steps = 0;
  const dispose = bindWalkInput(canvas as unknown as HTMLCanvasElement, {
    stepCamera(direction) {
      steps += direction;
    },
    lookAround(yaw: number, pitch: number) {
      turns.push([yaw, pitch]);
    },
  });
  const send = (type: string, extra = {}) =>
    canvas.dispatchEvent(
      Object.assign(new Event(type, { cancelable: true }), {
        pointerId: 1,
        isPrimary: true,
        pointerType: "mouse",
        button: 0,
        buttons: 1,
        clientX: 100,
        clientY: 100,
        ...extra,
      }),
    );
  try {
    send("pointerdown", { button: 2 });
    send("pointermove", { clientX: 300 });
    assert.equal(turns.length, 0);
    send("pointerdown");
    send("pointerdown", { pointerId: 2, isPrimary: false });
    send("pointermove", { pointerId: 2, clientX: 700 });
    send("pointermove", { clientX: 300, clientY: 200 });
    assert.equal(turns.length, 1);
    assert.ok(Math.abs(turns[0][0] + Math.PI / 2) < 1e-12);
    assert.ok(Math.abs(turns[0][1] + Math.PI / 4) < 1e-12);
    assert.equal(steps, 0);
    send("pointerup");
    send("pointermove", { clientX: 500 });
    assert.equal(turns.length, 1);
    send("pointerdown");
    send("pointercancel");
    send("pointermove", { clientX: 600 });
    assert.equal(turns.length, 1);
    send("pointerdown");
    send("lostpointercapture");
    send("pointermove", { clientX: 600 });
    assert.equal(turns.length, 1);
    send("pointerdown");
    dispose();
    send("pointermove", { clientX: 600 });
    assert.equal(turns.length, 1);
    assert.equal(captured, undefined);
  } finally {
    dispose();
    Object.defineProperty(globalThis, "window", { configurable: true, value: prior });
  }
});
