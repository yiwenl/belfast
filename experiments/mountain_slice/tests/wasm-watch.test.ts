import { test } from "node:test";
import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";
import { mkdtemp, mkdir, writeFile, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createBuildQueue, watchSources } from "../scripts/wasm-watch.mjs";

async function until(predicate: () => boolean) {
  const deadline = Date.now() + 3000;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error("Timed out waiting for watcher");
    await delay(10);
  }
}

test("coalesces saves, serializes builds and rebuilds edits made during a build", async () => {
  let starts = 0;
  const releases: (() => void)[] = [];
  const queue = createBuildQueue(
    async () => {
      starts++;
      await new Promise<void>((resolve) => releases.push(resolve));
    },
    { debounceMs: 10 },
  );
  try {
    queue.request();
    queue.request();
    queue.request();
    await until(() => starts === 1);
    queue.request();
    queue.request();
    await delay(30);
    assert.equal(starts, 1, "builds must not overlap");
    releases.shift()!();
    await until(() => starts === 2);
    releases.shift()!();
    await delay(30);
    assert.equal(starts, 2, "one trailing build for all pending saves");
  } finally {
    queue.stop();
    releases.forEach((release) => release());
  }
});

test("continues after a failed build and cancels queued work on shutdown", async () => {
  let starts = 0;
  let errors = 0;
  const queue = createBuildQueue(
    async () => {
      if (++starts === 1) throw new Error("invalid shader");
    },
    { debounceMs: 5, onError: () => errors++ },
  );
  queue.request();
  await until(() => errors === 1);
  queue.request();
  await until(() => starts === 2);
  queue.request();
  queue.stop();
  queue.request();
  await delay(30);
  assert.equal(starts, 2);
});

test("watches nested Rust/WGSL and atomic saves without watching generated files", async () => {
  const directory = await mkdtemp(join(tmpdir(), "belfast-watch-"));
  const events: string[] = [];
  const close = watchSources([directory], (file: string) => events.push(file));
  try {
    await mkdir(join(directory, "nested"));
    await writeFile(join(directory, "nested", "main.rs"), "// rust");
    await until(() => events.some((file) => file.endsWith("main.rs")));
    await writeFile(join(directory, "nested", "shader.tmp"), "// shader");
    await rename(join(directory, "nested", "shader.tmp"), join(directory, "nested", "sky.wgsl"));
    await until(() => events.some((file) => file.endsWith("sky.wgsl")));
    await writeFile(join(directory, "ignored.wasm"), "generated");
    await writeFile(join(directory, "ignored.js"), "generated");
    await delay(30);
    assert.ok(events.every((file) => /\.(rs|wgsl)$/.test(file)));
  } finally {
    close();
    await rm(directory, { recursive: true, force: true });
  }
});
