import assert from "node:assert/strict";
import { test } from "node:test";
import {
  createPromptController,
  type PromptState,
  type PromptHandlers,
} from "../src/prompt-panel.ts";
import type { GeneratedImage } from "../src/api-client.ts";
import type { PaletteColor } from "../src/palette.ts";

const colors: PaletteColor[] = [[20, 180, 80]];

const image: GeneratedImage = {
  blob: new Blob(),
  promptId: "id",
  effectivePrompt: "Forest",
  seed: 1,
  steps: 1,
  receivedMs: 50,
  completedMs: 60,
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function panel(handlers: PromptHandlers) {
  const states: PromptState[] = [];
  const controller = createPromptController((state) => states.push(state), handlers);
  return {
    ...controller,
    states,
    get state() {
      return states.at(-1)!;
    },
  };
}

test("empty text never submits; busy covers both generation and image application", async () => {
  const generated = deferred<GeneratedImage>();
  const applied = deferred<PaletteColor[]>();
  const texts: string[] = [];
  const p = panel({
    generate: async (text) => {
      texts.push(text);
      return generated.promise;
    },
    applyImage: () => applied.promise,
  });
  await p.submit("  ");
  assert.equal(texts.length, 0);
  assert.equal(p.state.phase, "error");
  const pending = p.submit("  Forest  ");
  assert.equal(p.state.phase, "generating");
  assert.equal(p.state.busy, true);
  await p.submit("Another");
  assert.deepEqual(texts, ["Forest"]);
  generated.resolve(image);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(p.state.phase, "applying");
  assert.equal(p.state.busy, true);
  await p.submit("Another");
  assert.deepEqual(texts, ["Forest"]);
  applied.resolve(colors);
  await pending;
  assert.equal(p.state.phase, "ready");
  assert.equal(p.state.busy, false);
});

test("failure unlocks without retrying and switches timeout only after successful generation", async () => {
  const timeouts: number[] = [];
  let fail = true;
  const p = panel({
    generate: async (_text, _signal, timeout) => {
      timeouts.push(timeout);
      if (fail) throw new Error("Server busy");
      return image;
    },
    applyImage: async () => colors,
  });
  await p.submit("Forest");
  assert.equal(p.state.phase, "error");
  assert.equal(p.state.busy, false);
  assert.equal(timeouts.length, 1);
  fail = false;
  await p.submit("Ocean");
  await p.submit("Desert");
  assert.deepEqual(timeouts, [120000, 120000, 30000]);
});

test("image application failure is recoverable", async () => {
  const p = panel({
    generate: async () => image,
    applyImage: async () => {
      throw new Error("Invalid PNG");
    },
  });
  await p.submit("Forest");
  assert.equal(p.state.phase, "error");
  assert.equal(p.state.busy, false);
  assert.match(p.state.message, /Invalid PNG/);
});

test("dispose aborts generation and prevents late results or errors from updating the view", async () => {
  for (const failure of [false, true]) {
    const generated = deferred<GeneratedImage>();
    let signal!: AbortSignal;
    let applications = 0;
    const p = panel({
      generate: async (_text, s) => {
        signal = s;
        return generated.promise;
      },
      applyImage: async () => {
        applications++;
        return colors;
      },
    });
    const pending = p.submit("Forest");
    p.dispose();
    p.dispose();
    const count = p.states.length;
    assert.equal(signal.aborted, true);
    if (failure) generated.reject(new Error("Socket closed"));
    else generated.resolve(image);
    await pending;
    await p.submit("Another");
    assert.equal(applications, 0);
    assert.equal(p.states.length, count);
  }
});

test("dispose during application prevents ready state after the renderer has been released", async () => {
  const applied = deferred<PaletteColor[]>();
  const p = panel({ generate: async () => image, applyImage: () => applied.promise });
  const pending = p.submit("Forest");
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(p.state.phase, "applying");
  p.dispose();
  const count = p.states.length;
  applied.resolve(colors);
  await pending;
  assert.equal(p.states.length, count);
});
