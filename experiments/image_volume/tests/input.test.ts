import assert from "node:assert/strict";
import test from "node:test";
import { bindOrbitInput, type OrbitTarget } from "../web/input.ts";

// Browser event/capture boundary; the actual controller remains Rust-owned.
class CanvasEvents extends EventTarget {
  clientHeight = 400;
  captured = new Set<number>();
  getBoundingClientRect() {
    return { left: 20, top: 40, width: 600, height: 400 };
  }
  setPointerCapture(id: number) {
    this.captured.add(id);
  }
  hasPointerCapture(id: number) {
    return this.captured.has(id);
  }
  releasePointerCapture(id: number) {
    this.captured.delete(id);
  }
}

function emit(target: EventTarget, type: string, values: Record<string, unknown> = {}) {
  target.dispatchEvent(Object.assign(new Event(type, { cancelable: true }), values));
}

test("forwards CSS coordinates, pan modifiers and wheel units; cancellation and cleanup end input", () => {
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const windowEvents = new EventTarget();
  Object.defineProperty(globalThis, "window", { configurable: true, value: windowEvents });
  const canvas = new CanvasEvents();
  const commands: unknown[][] = [];
  const app: OrbitTarget = {
    pointerDown: (...args) => commands.push(["down", ...args]),
    pointerMove: (...args) => commands.push(["move", ...args]),
    pointerUp: () => commands.push(["up"]),
    scroll: (...args) => commands.push(["scroll", ...args]),
  };
  const dispose = bindOrbitInput(canvas as unknown as HTMLCanvasElement, app);
  try {
    emit(canvas, "pointerdown", {
      pointerId: 1,
      button: 0,
      shiftKey: true,
      clientX: 120,
      clientY: 90,
    });
    emit(canvas, "pointermove", { pointerId: 1, buttons: 1, clientX: 150, clientY: 110 });
    assert.deepEqual(commands, [
      ["down", 100, 50, 0, true],
      ["move", 130, 70, 600, 400],
    ]);
    emit(windowEvents, "blur");
    assert.deepEqual(commands.at(-1), ["up"]);
    assert.equal(canvas.captured.size, 0);
    emit(canvas, "pointermove", { pointerId: 1, buttons: 1, clientX: 200, clientY: 200 });
    assert.deepEqual(commands.at(-1), ["up"]);

    emit(canvas, "wheel", { deltaMode: 1, deltaY: 2 });
    assert.deepEqual(commands.at(-1), ["scroll", 32]);
    emit(canvas, "wheel", { deltaMode: 2, deltaY: -1 });
    assert.deepEqual(commands.at(-1), ["scroll", -400]);

    emit(canvas, "pointerdown", {
      pointerId: 2,
      button: 1,
      shiftKey: false,
      clientX: 30,
      clientY: 50,
    });
    assert.deepEqual(commands.at(-1), ["down", 10, 10, 1, false]);
    emit(canvas, "pointercancel", { pointerId: 2 });
    assert.deepEqual(commands.at(-1), ["up"]);
    assert.equal(canvas.captured.size, 0);
    dispose();
    const count = commands.length;
    emit(canvas, "wheel", { deltaMode: 0, deltaY: 100 });
    emit(canvas, "pointerdown", { pointerId: 3, button: 0, clientX: 50, clientY: 50 });
    assert.equal(commands.length, count);
  } finally {
    dispose();
    if (previousWindow) Object.defineProperty(globalThis, "window", previousWindow);
    else Reflect.deleteProperty(globalThis, "window");
  }
});
