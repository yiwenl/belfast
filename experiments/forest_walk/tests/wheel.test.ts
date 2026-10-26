import assert from "node:assert/strict";
import test from "node:test";
import { WheelStepper } from "../web/wheel.ts";

test("mouse wheel selects exactly one adjacent frame per event", () => {
  const wheel = new WheelStepper();
  assert.equal(wheel.push(120, 0, 800), 1);
  assert.equal(wheel.push(1200, 0, 800), 1);
  assert.equal(wheel.push(-120, 0, 800), -1);
});

test("trackpad accumulates small motion and reversals cancel prior motion", () => {
  const wheel = new WheelStepper();
  assert.equal(wheel.push(40, 0, 800), 0);
  assert.equal(wheel.push(40, 0, 800), 1);
  assert.equal(wheel.push(60, 0, 800), 0);
  assert.equal(wheel.push(-40, 0, 800), 0);
  assert.equal(wheel.push(-40, 0, 800), -1);
});

test("normalizes line/page deltas and ignores pinch or invalid input", () => {
  const wheel = new WheelStepper();
  assert.equal(wheel.push(5, 1, 800), 1);
  assert.equal(wheel.push(-1, 2, 800), -1);
  assert.equal(wheel.push(120, 0, 800, true), 0);
  assert.equal(wheel.push(NaN, 0, 800), 0);
  assert.equal(wheel.push(0, 0, 800), 0);
});
