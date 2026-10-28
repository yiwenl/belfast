// Adapted from http://192.168.1.201:8189/client.mjs; adds AbortSignal lifecycle cleanup.
// getRandomValues also works on plain HTTP LAN pages (randomUUID requires HTTPS).
function requestUUID() {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 15) | 64;
  bytes[8] = (bytes[8] & 63) | 128;
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
// Browser or Node >=22. No npm dependencies. One request per client at a time.
export class ParticleClient {
  constructor({
    baseUrl = "http://127.0.0.1:8189",
    fetch = globalThis.fetch,
    WebSocket = globalThis.WebSocket,
  } = {}) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.fetch = fetch.bind(globalThis);
    this.WebSocket = WebSocket;
    this.busy = false;
  }
  async cancel(promptId) {
    const errors = [];
    // Attempt every cleanup action even if an earlier HTTP operation fails.
    for (const [path, body] of [
      ["/queue", { delete: [promptId] }],
      ["/interrupt", { prompt_id: promptId }],
      ["/queue", { delete: [promptId] }],
    ]) {
      try {
        const response = await this.fetch(this.baseUrl + path, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
          signal: AbortSignal.timeout(2000),
        });
        if (!response.ok) throw new Error(`${path} HTTP ${response.status}`);
      } catch (error) {
        errors.push(error.message);
      }
    }
    if (errors.length) throw new Error(errors.join("; "));
  }
  async generate(
    template,
    text,
    {
      steps = 1,
      seed = crypto.getRandomValues(new Uint32Array(1))[0],
      timeoutMs = 30000,
      onEvent = () => {},
      signal,
    } = {},
  ) {
    if (this.busy) throw new Error("Client busy: only one request is allowed");
    this.busy = true;
    let socket,
      timer,
      submitted = false,
      finished = false,
      currentNode = null,
      previousNode = null,
      nodeStart = 0;
    const started = performance.now(),
      promptId = requestUUID(),
      clientId = requestUUID();
    const controller = new AbortController(),
      nodeMs = {},
      cachedNodes = [];
    let image, receivedMs, resolveResult, rejectResult;
    const completion = new Promise((resolve, reject) => {
      resolveResult = resolve;
      rejectResult = reject;
    });
    // Observe early socket failures while HTTP submission is still in flight.
    completion.catch(() => {});
    const fail = (error) => {
      if (!finished) {
        finished = true;
        rejectResult(error);
        controller.abort(error);
      }
    };
    const onAbort = () => {
      controller.abort(signal.reason);
      fail(signal.reason);
    };
    try {
      signal?.throwIfAborted();
      signal?.addEventListener("abort", onAbort, { once: true });
      timer = setTimeout(() => {
        const error = new Error(`Generation timeout after ${timeoutMs} ms (${promptId})`);
        controller.abort(error);
        fail(error);
      }, timeoutMs);
      const qResponse = await this.fetch(this.baseUrl + "/queue", { signal: controller.signal });
      if (!qResponse.ok) throw new Error(`Queue check HTTP ${qResponse.status}`);
      const q = await qResponse.json();
      if (q.queue_running.length || q.queue_pending.length)
        throw new Error("Server busy: retry after its current request finishes");
      const workflow = structuredClone(template);
      workflow["6"].inputs.text = text;
      workflow["3"].inputs.noise_seed = seed;
      workflow["10"].inputs.steps = steps;
      socket = new this.WebSocket(this.baseUrl.replace(/^http/, "ws") + "/ws?clientId=" + clientId);
      socket.binaryType = "arraybuffer";
      socket.addEventListener("message", (event) => {
        if (finished) return;
        try {
          if (typeof event.data === "string") {
            const message = JSON.parse(event.data),
              d = message.data;
            if (d?.prompt_id !== promptId) return;
            onEvent(message);
            if (message.type === "executing") {
              const now = performance.now();
              if (previousNode !== null)
                nodeMs[previousNode] = (nodeMs[previousNode] || 0) + now - nodeStart;
              currentNode = d.node;
              previousNode = d.node;
              nodeStart = now;
            }
            if (message.type === "execution_cached") cachedNodes.push(...d.nodes);
            if (message.type === "execution_error" || message.type === "execution_interrupted")
              fail(new Error(d.exception_message || message.type));
            if (
              message.type === "execution_success" ||
              (message.type === "executing" && d.node === null)
            ) {
              if (previousNode !== null) {
                nodeMs[previousNode] = (nodeMs[previousNode] || 0) + performance.now() - nodeStart;
                previousNode = null;
              }
              if (!image) {
                fail(new Error("Execution finished without a final WebSocket image"));
                return;
              }
              finished = true;
              resolveResult({
                blob: image,
                promptId,
                seed,
                steps,
                receivedMs,
                completedMs: performance.now() - started,
                nodeMs,
                cachedNodes,
              });
            }
          } else if (currentNode === "9" && event.data instanceof ArrayBuffer) {
            const frame = new DataView(event.data);
            // Official SaveImageWebsocket: big-endian event=1, format=2 (PNG).
            // Other binary events/previews must not become a texture.
            if (frame.byteLength > 8 && frame.getUint32(0) === 1 && frame.getUint32(4) === 2) {
              image = new Blob([event.data.slice(8)], { type: "image/png" });
              receivedMs = performance.now() - started;
            }
          }
        } catch (error) {
          fail(error);
        }
      });
      socket.addEventListener("error", () => fail(new Error("WebSocket connection failed")));
      socket.addEventListener("close", () => {
        if (!finished) fail(new Error("WebSocket closed before result"));
      });
      await new Promise((resolve, reject) => {
        socket.addEventListener("open", resolve, { once: true });
        controller.signal.addEventListener("abort", () => reject(controller.signal.reason), {
          once: true,
        });
        if (controller.signal.aborted) reject(controller.signal.reason);
      });
      submitted = true;
      const response = await this.fetch(this.baseUrl + "/prompt", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prompt: workflow, client_id: clientId, prompt_id: promptId }),
        signal: controller.signal,
      });
      const data = await response.json();
      if (!response.ok) throw new Error(`Prompt HTTP ${response.status}: ${JSON.stringify(data)}`);
      if (data.prompt_id !== promptId)
        throw new Error("Server did not preserve request prompt_id; requires compatible ComfyUI");
      return await completion;
    } catch (error) {
      finished = true;
      const failure = new Error(error instanceof Error ? error.message : String(error));
      if (submitted) {
        try {
          await this.cancel(promptId);
        } catch (cleanupError) {
          failure.message += `; cancellation unconfirmed: ${cleanupError.message}`;
        }
      }
      throw failure;
    } finally {
      finished = true;
      signal?.removeEventListener("abort", onAbort);
      clearTimeout(timer);
      controller.abort();
      socket?.close();
      this.busy = false;
    }
  }
}
