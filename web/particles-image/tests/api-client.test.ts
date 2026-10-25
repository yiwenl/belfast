import assert from "node:assert/strict";
import { test } from "node:test";
import { ParticleImageClient, type Workflow } from "../src/api-client.ts";

const workflow: Workflow = {
  "6": { class_type: "CLIPTextEncode", inputs: { text: "original" } },
  "3": { class_type: "SamplerCustom", inputs: { noise_seed: 42 } },
  "10": { class_type: "SDTurboScheduler", inputs: { steps: 4 } },
};
const options = { timeoutMs: 500, seed: 123 };
function pngFrame(marker = 7, event = 1, format = 2) {
  const bytes = new Uint8Array(17);
  const view = new DataView(bytes.buffer);
  view.setUint32(0, event);
  view.setUint32(4, format);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, marker], 8);
  return bytes.buffer;
}
function fixture(
  config: {
    queue?: unknown;
    queueStatus?: number;
    openError?: boolean;
    prompt?: (f: ReturnType<typeof fixture>, body: any, signal: AbortSignal) => Promise<Response>;
    cleanup?: (path: string, body: any) => Promise<Response>;
  } = {},
) {
  const calls: { path: string; body: any }[] = [];
  let socket!: FakeSocket;
  class FakeSocket extends EventTarget {
    binaryType = "blob";
    closed = false;
    constructor(readonly url: string) {
      super();
      socket = this;
      queueMicrotask(() => this.dispatchEvent(new Event(config.openError ? "error" : "open")));
    }
    message(data: unknown) {
      this.dispatchEvent(new MessageEvent("message", { data }));
    }
    json(type: string, promptId: string, extra = {}) {
      this.message(JSON.stringify({ type, data: { prompt_id: promptId, ...extra } }));
    }
    close() {
      this.closed = true;
      this.dispatchEvent(new Event("close"));
    }
  }
  const client = new ParticleImageClient({
    baseUrl: "http://localhost:5173/comfy/",
    workflow,
    WebSocket: FakeSocket as unknown as typeof WebSocket,
    fetch: (async (url: string, init?: RequestInit) => {
      const path = new URL(url).pathname;
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      calls.push({ path, body });
      if (!body)
        return Response.json(config.queue ?? { queue_running: [], queue_pending: [] }, {
          status: config.queueStatus ?? 200,
        });
      if (path === "/comfy/prompt") {
        if (config.prompt) return config.prompt(result, body, init!.signal as AbortSignal);
        socket.json("executing", body.prompt_id, { node: "9" });
        socket.message(pngFrame());
        socket.json("execution_success", body.prompt_id);
        return Response.json({ prompt_id: body.prompt_id, number: 0, node_errors: {} });
      }
      return config.cleanup ? config.cleanup(path, body) : Response.json({});
    }) as typeof fetch,
  });
  const result = {
    client,
    calls,
    get socket() {
      return socket;
    },
  };
  return result;
}

test("accepts final PNG arriving before POST response and preserves the workflow template", async () => {
  const f = fixture();
  const result = await f.client.generate("  A blue ocean  ", options);
  assert.equal(result.blob.type, "image/png");
  assert.deepEqual(
    [...new Uint8Array(await result.blob.arrayBuffer())],
    [137, 80, 78, 71, 13, 10, 26, 10, 7],
  );
  const body = f.calls[1].body;
  assert.match(body.prompt["6"].inputs.text, /^A blue ocean, \(.+:0\.65\)$/);
  assert.equal(result.effectivePrompt, body.prompt["6"].inputs.text);
  assert.equal(body.prompt["3"].inputs.noise_seed, 123);
  assert.equal(body.prompt["10"].inputs.steps, 1);
  assert.notEqual(body.client_id, body.prompt_id);
  assert.match(f.socket.url, /^ws:\/\/localhost:5173\/comfy\/ws\?clientId=/);
  assert.equal(new URL(f.socket.url).searchParams.get("clientId"), body.client_id);
  assert.equal(workflow["6"].inputs.text, "original");
  assert.equal(workflow["10"].inputs.steps, 4);
  assert.equal(f.client.busy, false);
  assert.equal(f.socket.closed, true);
  assert.equal(f.calls.length, 2);
});

test("each submission draws one subtle suffix while preserving the original prompt", async (t) => {
  const samples = [0, 0.999999];
  t.mock.method(Math, "random", () => samples.shift() ?? 0.5);
  const f = fixture();
  const original = "A (deep red:1.3) rose";
  const first = await f.client.generate(original, options);
  const second = await f.client.generate(original, options);
  const prompts = f.calls
    .filter((call) => call.path === "/comfy/prompt")
    .map((call) => call.body.prompt["6"].inputs.text as string);
  assert.equal(samples.length, 0);
  assert.equal(prompts.length, 2);
  assert.notEqual(prompts[0], prompts[1]);
  for (const prompt of prompts) {
    assert.ok(prompt.startsWith(`${original}, (`));
    assert.ok(prompt.endsWith(":0.65)"));
    assert.equal(prompt.split(original).length, 2);
  }
  assert.equal(first.effectivePrompt, prompts[0]);
  assert.equal(second.effectivePrompt, prompts[1]);
  assert.equal(workflow["6"].inputs.text, "original");
});

test("ignores previews and other prompts, accepting only node 9 PNG and null-node completion", async () => {
  const f = fixture({
    prompt: async (f, body) => {
      f.socket.json("executing", "another-prompt", { node: "9" });
      f.socket.message(pngFrame(1));
      f.socket.json("execution_error", "another-prompt");
      f.socket.json("executing", body.prompt_id, { node: "3" });
      f.socket.message(pngFrame(2));
      f.socket.json("executing", body.prompt_id, { node: "9" });
      f.socket.message(pngFrame(3, 1, 1));
      f.socket.message(new ArrayBuffer(4));
      f.socket.message(pngFrame(4));
      f.socket.json("executing", body.prompt_id, { node: null });
      return Response.json({ prompt_id: body.prompt_id, number: 0, node_errors: {} });
    },
  });
  const result = await f.client.generate("Forest", options);
  assert.equal(new Uint8Array(await result.blob.arrayBuffer())[8], 4);
});

test("blank text and an already aborted request do not contact the server", async () => {
  const f = fixture();
  await assert.rejects(f.client.generate("   ", options), /prompt/i);
  await assert.rejects(
    f.client.generate("Forest", { ...options, signal: AbortSignal.abort() }),
    /abort/i,
  );
  assert.equal(f.calls.length, 0);
  assert.equal(f.client.busy, false);
});

for (const queue of [
  { queue_running: [[1]], queue_pending: [] },
  { queue_running: [], queue_pending: [[1]] },
]) {
  test(`rejects occupied server queue ${JSON.stringify(queue)}`, async () => {
    const f = fixture({ queue });
    await assert.rejects(f.client.generate("Forest", options), /Server busy/);
    assert.equal(f.calls.length, 1);
    assert.equal(f.client.busy, false);
  });
}

test("invalid queue and HTTP failure never submit a prompt", async () => {
  for (const config of [{ queue: {} }, { queueStatus: 503 }]) {
    const f = fixture(config);
    await assert.rejects(f.client.generate("Forest", options), /queue/i);
    assert.equal(f.calls.length, 1);
    assert.equal(f.client.busy, false);
  }
});

test("socket failure before open rejects promptly and unlocks without cancelling anything", async () => {
  const f = fixture({ openError: true });
  await assert.rejects(f.client.generate("Forest", options), /WebSocket connection/);
  assert.equal(f.calls.length, 1);
  assert.equal(f.socket.closed, true);
  assert.equal(f.client.busy, false);
});

for (const failure of [
  "execution_error",
  "execution_interrupted",
  "close",
  "no-image",
  "http",
  "wrong-id",
]) {
  test(`cleans up only its own prompt on ${failure}`, async () => {
    const f = fixture({
      prompt: async (f, body) => {
        if (failure === "close") f.socket.close();
        else if (failure === "no-image") f.socket.json("execution_success", body.prompt_id);
        else if (failure.startsWith("execution_"))
          f.socket.json(failure, body.prompt_id, { exception_message: "GPU failure" });
        return Response.json(
          {
            prompt_id: failure === "wrong-id" ? "other" : body.prompt_id,
            number: 0,
            node_errors: {},
          },
          { status: failure === "http" ? 400 : 200 },
        );
      },
    });
    await assert.rejects(f.client.generate("Forest", options));
    const id = f.calls[1].body.prompt_id;
    assert.deepEqual(f.calls.slice(2), [
      { path: "/comfy/queue", body: { delete: [id] } },
      { path: "/comfy/interrupt", body: { prompt_id: id } },
      { path: "/comfy/queue", body: { delete: [id] } },
    ]);
    assert.equal(f.client.busy, false);
    assert.equal(f.socket.closed, true);
  });
}

test("timeout keeps client locked through cleanup, attempts every cancellation, and does not retry", async () => {
  let finishCleanup!: () => void;
  const f = fixture({
    prompt: async (_f, body) =>
      Response.json({ prompt_id: body.prompt_id, number: 0, node_errors: {} }),
    cleanup: async (_path, body) => {
      if (body.prompt_id) {
        await new Promise<void>((resolve) => {
          finishCleanup = resolve;
        });
      }
      return new Response("cleanup failed", { status: 500 });
    },
  });
  const pending = f.client.generate("Forest", { ...options, timeoutMs: 15 });
  const rejection = assert.rejects(pending, /timeout.*cancellation unconfirmed/i);
  await assert.rejects(f.client.generate("Another", options), /Client busy/);
  while (!finishCleanup) await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(f.client.busy, true);
  finishCleanup();
  await rejection;
  assert.equal(f.calls.filter((call) => call.path === "/comfy/prompt").length, 1);
  assert.equal(f.calls.length, 5);
  assert.equal(f.client.busy, false);
});

test("abort while POST is in flight still cancels the possibly accepted request", async () => {
  const abort = new AbortController();
  const f = fixture({
    prompt: async (_f, _body, signal) => {
      queueMicrotask(() => abort.abort());
      return new Promise((_resolve, reject) =>
        signal.addEventListener("abort", () => reject(signal.reason), { once: true }),
      );
    },
  });
  await assert.rejects(f.client.generate("Forest", { ...options, signal: abort.signal }), /abort/i);
  assert.equal(f.calls.length, 5);
  assert.equal(f.client.busy, false);
});
