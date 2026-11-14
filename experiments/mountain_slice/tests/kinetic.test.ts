import assert from "node:assert/strict";
import test from "node:test";
import { createSkeletonAdapter, type KineticPose } from "../web/kinetic.ts";

function setup() {
  let received: { points: Float32Array; time: number } | undefined;
  const adapter = createSkeletonAdapter({
    setSkeleton(points, time) {
      received = { points: points.slice(), time };
    },
    clearSkeleton() {
      received = undefined;
    },
  });
  return {
    adapter,
    get received() {
      return received;
    },
  };
}

function pose(): KineticPose {
  return { keypoints3D: Array.from({ length: 33 }, () => ({ x: 1, y: 2, z: -3, score: 0.5 })) };
}

test("packs 33 points in order, preserves mirrored X, converts Y/Z and timestamps", () => {
  const sink = setup();
  const input = pose();
  input.keypoints3D[1].x = -4;
  sink.adapter.update([input], 1234);
  assert.equal(sink.received?.time, 1234);
  assert.equal(sink.received?.points.length, 132);
  assert.deepEqual(Array.from(sink.received!.points.slice(0, 8)), [1, -2, 3, 0.5, -4, -2, 3, 0.5]);
});

test("clears missing or malformed tracking instead of keeping an old pose", () => {
  const sink = setup();
  for (const invalid of [
    [],
    [{ keypoints3D: [] }],
    [{ keypoints3D: pose().keypoints3D.map((p) => ({ ...p, x: NaN })) }],
  ]) {
    sink.adapter.update([pose()], 1000);
    assert.ok(sink.received);
    sink.adapter.update(invalid, 1001);
    assert.equal(sink.received, undefined);
  }
});

test("missing confidence is untrusted and explicit clear removes input", () => {
  const sink = setup();
  sink.adapter.update(
    [{ keypoints3D: pose().keypoints3D.map(({ x, y, z }) => ({ x, y, z })) }],
    1000,
  );
  assert.equal(sink.received?.points[3], 0);
  sink.adapter.clear();
  assert.equal(sink.received, undefined);
});
