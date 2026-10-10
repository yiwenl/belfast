export type Workflow = Record<string, { class_type: string; inputs: Record<string, unknown> }>;
export type ComfyEvent = {
  type: string;
  data: { prompt_id?: string; node?: string | null; [key: string]: unknown };
};
export type GeneratedImage = {
  blob: Blob;
  promptId: string;
  seed: number;
  steps: number;
  receivedMs: number;
  completedMs: number;
};
export interface GenerateOptions {
  timeoutMs: number;
  signal?: AbortSignal;
  seed?: number;
  onEvent?: (event: ComfyEvent) => void;
}
export interface ClientOptions {
  baseUrl: string;
  workflow: Workflow;
  fetch?: typeof fetch;
  WebSocket?: typeof WebSocket;
}
export class ParticleImageClient {
  private active = false;
  private readonly baseUrl: string;
  private readonly fetch: typeof fetch;
  private readonly Socket: typeof WebSocket;

  constructor(private readonly options: ClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.fetch = (options.fetch ?? globalThis.fetch).bind(globalThis);
    this.Socket = options.WebSocket ?? globalThis.WebSocket;
  }

  get busy(): boolean {
    return this.active;
  }

  // Adapted from the server's particle-api-starter/client.mjs (2026-10-10).
  // Each request owns its socket: binary ComfyUI frames have no prompt ID.
  async generate(text: string, options: GenerateOptions): Promise<GeneratedImage> {
    if (this.active) throw new Error("Client busy: only one request is allowed");
    if (!text.trim()) throw new Error("Enter a prompt before generating");
    options.signal?.throwIfAborted();
    if (!Number.isFinite(options.timeoutMs) || options.timeoutMs <= 0) {
      throw new Error("Generation timeout must be a positive number");
    }
    const seed = options.seed ?? crypto.getRandomValues(new Uint32Array(1))[0];
    if (!Number.isSafeInteger(seed) || seed < 0)
      throw new Error("Seed must be a non-negative integer");
    const promptId = crypto.randomUUID();
    const clientId = crypto.randomUUID();
    const started = performance.now();
    const controller = new AbortController();
    const listeners = new AbortController();
    let socket: WebSocket | undefined;
    let submitted = false;
    let finished = false;
    let currentNode: string | null = null;
    let image: Blob | undefined;
    let receivedMs = 0;
    let resolveResult!: (result: GeneratedImage) => void;
    let rejectResult!: (error: Error) => void;
    const completion = new Promise<GeneratedImage>((resolve, reject) => {
      resolveResult = resolve;
      rejectResult = reject;
    });
    // Errors can arrive while HTTP submission is still pending.
    void completion.catch(() => {});
    const fail = (reason: unknown) => {
      const error = asError(reason);
      if (!finished) {
        finished = true;
        rejectResult(error);
      }
      controller.abort(error);
    };
    const abort = () => fail(options.signal?.reason ?? new DOMException("Aborted", "AbortError"));
    options.signal?.addEventListener("abort", abort, { once: true });
    const timer = setTimeout(
      () => fail(new Error(`Generation timeout after ${options.timeoutMs} ms (${promptId})`)),
      options.timeoutMs,
    );
    this.active = true;

    try {
      const qResponse = await this.fetch(`${this.baseUrl}/queue`, { signal: controller.signal });
      if (!qResponse.ok) throw new Error(`Queue check HTTP ${qResponse.status}`);
      const queue = await qResponse.json();
      if (!Array.isArray(queue?.queue_running) || !Array.isArray(queue?.queue_pending)) {
        throw new Error("Invalid queue response");
      }
      if (queue.queue_running.length || queue.queue_pending.length)
        throw new Error("Server busy: retry after its current request finishes");
      controller.signal.throwIfAborted();

      const workflow = structuredClone(this.options.workflow);
      workflow["6"].inputs.text = text.trim();
      workflow["3"].inputs.noise_seed = seed;
      workflow["10"].inputs.steps = 1;
      socket = new this.Socket(`${this.baseUrl.replace(/^http/, "ws")}/ws?clientId=${clientId}`);
      socket.binaryType = "arraybuffer";
      socket.addEventListener(
        "message",
        (event: MessageEvent) => {
          if (finished) return;
          try {
            if (typeof event.data === "string") {
              const message = JSON.parse(event.data);
              const data = message?.data;
              if (data?.prompt_id !== promptId || typeof message.type !== "string") return;
              options.onEvent?.(message as ComfyEvent);
              if (message.type === "executing") {
                currentNode = typeof data.node === "string" ? data.node : null;
              }
              if (message.type === "execution_error" || message.type === "execution_interrupted") {
                fail(
                  new Error(
                    typeof data.exception_message === "string"
                      ? data.exception_message
                      : message.type,
                  ),
                );
                return;
              }
              if (
                message.type === "execution_success" ||
                (message.type === "executing" && data.node === null)
              ) {
                if (!image) throw new Error("Execution finished without a final WebSocket image");
                finished = true;
                resolveResult({
                  blob: image,
                  promptId,
                  seed,
                  steps: 1,
                  receivedMs,
                  completedMs: performance.now() - started,
                });
              }
            } else if (
              currentNode === "9" &&
              event.data instanceof ArrayBuffer &&
              event.data.byteLength > 8
            ) {
              const frame = new DataView(event.data);
              if (frame.getUint32(0) === 1 && frame.getUint32(4) === 2) {
                image = new Blob([event.data.slice(8)], { type: "image/png" });
                receivedMs = performance.now() - started;
              }
            }
          } catch (error) {
            fail(error);
          }
        },
        { signal: listeners.signal },
      );
      socket.addEventListener("error", () => fail(new Error("WebSocket connection failed")), {
        signal: listeners.signal,
      });
      socket.addEventListener(
        "close",
        () => {
          if (!finished) fail(new Error("WebSocket closed before result"));
        },
        { signal: listeners.signal },
      );
      await new Promise<void>((resolve, reject) => {
        socket!.addEventListener("open", () => resolve(), { once: true, signal: listeners.signal });
        controller.signal.addEventListener("abort", () => reject(controller.signal.reason), {
          once: true,
          signal: listeners.signal,
        });
        if (controller.signal.aborted) reject(controller.signal.reason);
      });
      controller.signal.throwIfAborted();
      // A failed/aborted fetch can still have been accepted by the server.
      submitted = true;
      const response = await this.fetch(`${this.baseUrl}/prompt`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prompt: workflow, client_id: clientId, prompt_id: promptId }),
        signal: controller.signal,
      });
      if (!response.ok) throw new Error(`Prompt HTTP ${response.status}: ${await response.text()}`);
      const result = await response.json();
      if (result.prompt_id !== promptId)
        throw new Error("Server did not preserve request prompt_id");
      controller.signal.throwIfAborted();
      return await completion;
    } catch (reason) {
      const error = asError(reason);
      finished = true;
      controller.abort(error);
      if (submitted) {
        try {
          await this.cancel(promptId);
        } catch (cleanupError) {
          throw new Error(
            `${error.message}; cancellation unconfirmed: ${asError(cleanupError).message}`,
            { cause: error },
          );
        }
      }
      throw error;
    } finally {
      finished = true;
      clearTimeout(timer);
      options.signal?.removeEventListener("abort", abort);
      listeners.abort();
      controller.abort();
      socket?.close();
      this.active = false;
    }
  }

  private async cancel(promptId: string): Promise<void> {
    const errors: string[] = [];
    for (const [path, body] of [
      ["/queue", { delete: [promptId] }],
      ["/interrupt", { prompt_id: promptId }],
      ["/queue", { delete: [promptId] }],
    ] as const) {
      try {
        const response = await this.fetch(this.baseUrl + path, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
          signal: AbortSignal.timeout(2000),
        });
        if (!response.ok) throw new Error(`${path} HTTP ${response.status}`);
      } catch (error) {
        errors.push(asError(error).message);
      }
    }
    if (errors.length) throw new Error(errors.join("; "));
  }
}

function asError(reason: unknown): Error {
  return reason instanceof Error ? reason : new Error(String(reason));
}
