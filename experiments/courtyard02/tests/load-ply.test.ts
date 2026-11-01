import assert from "node:assert/strict";
import { test } from "node:test";
import { loadPlyBytes } from "../web/load-ply";

test("compressed responses use decoded body size, not compressed Content-Length", async (t) => {
  t.mock.method(
    globalThis,
    "fetch",
    async () =>
      new Response(new Uint8Array([1, 2, 3, 4]), {
        headers: { "content-length": "2", "content-encoding": "gzip" },
      }),
  );
  assert.deepEqual(
    await loadPlyBytes("/courtyard.ply", new AbortController().signal, () => {}),
    new Uint8Array([1, 2, 3, 4]),
  );
});

test("uncompressed downloads report progress and reject truncated payloads", async (t) => {
  t.mock.method(
    globalThis,
    "fetch",
    async () =>
      new Response(new Uint8Array([1, 2]), {
        headers: { "content-length": "4" },
      }),
  );
  const progress: number[] = [];
  await assert.rejects(
    loadPlyBytes("/courtyard.ply", new AbortController().signal, (n) => progress.push(n)),
    /truncated/,
  );
  assert.deepEqual(progress, [2]);
});
