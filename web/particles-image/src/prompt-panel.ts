import type { GeneratedImage } from "./api-client";
export interface PromptState {
  phase: "idle" | "generating" | "applying" | "ready" | "error";
  busy: boolean;
  message: string;
}
export interface PromptHandlers {
  generate(text: string, signal: AbortSignal, timeoutMs: number): Promise<GeneratedImage>;
  applyImage(blob: Blob): Promise<void>;
}
export function createPromptController(
  render: (state: PromptState) => void,
  handlers: PromptHandlers,
) {
  let busy = false;
  let disposed = false;
  let hasGenerated = false;
  let request: AbortController | undefined;
  render({ phase: "idle", busy: false, message: "輸入描述，為粒子換上新的配色。" });
  return {
    async submit(text: string): Promise<void> {
      if (busy || disposed) return;
      if (!text.trim()) {
        render({ phase: "error", busy: false, message: "請先輸入英文描述。" });
        return;
      }
      busy = true;
      request = new AbortController();
      const started = performance.now();
      render({
        phase: "generating",
        busy: true,
        message: hasGenerated ? "正在生成圖片…" : "正在生成圖片，首次載入可能需要較久。",
      });
      try {
        const result = await handlers.generate(
          text.trim(),
          request.signal,
          hasGenerated ? 30000 : 120000,
        );
        if (disposed) return;
        hasGenerated = true;
        render({ phase: "applying", busy: true, message: "正在套用配色…" });
        await handlers.applyImage(result.blob);
        if (disposed) return;
        render({
          phase: "ready",
          busy: false,
          message: `配色已更新 · ${((performance.now() - started) / 1000).toFixed(1)} 秒`,
        });
      } catch (error) {
        if (!disposed) render({ phase: "error", busy: false, message: formatError(error) });
      } finally {
        busy = false;
        request = undefined;
      }
    },
    dispose(): void {
      disposed = true;
      request?.abort();
    },
  };
}

function formatError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("cancellation unconfirmed"))
    return "等待已結束，但未確認伺服器已停止工作。請稍後再試。";
  if (message.includes("Server busy")) return "圖片服務正在處理其他工作，請稍後再試。";
  if (message.includes("Generation timeout")) return "生成逾時，已清理本次請求。你可以再試一次。";
  if (/WebSocket|Failed to fetch|fetch failed|NetworkError/.test(message))
    return "無法連線到圖片服務，請確認主機與區域網路連線，再試一次。";
  return `未能更新配色：${message.slice(0, 300)}`;
}

export function mountPromptPanel(root: HTMLElement, handlers: PromptHandlers): { dispose(): void } {
  const form = root.querySelector("form");
  const input = root.querySelector("textarea");
  const button = root.querySelector("button");
  const status = root.querySelector<HTMLElement>("[role=status]");
  const preview = root.querySelector<HTMLElement>(".image-preview");
  const previewImage = preview?.querySelector("img");
  if (!form || !input || !button || !status || !preview || !previewImage)
    throw new Error("Prompt panel is missing its form controls");
  let disposed = false;
  let previewUrl: string | undefined;
  const controller = createPromptController(
    (state) => {
      form.setAttribute("aria-busy", String(state.busy));
      input.disabled = button.disabled = state.busy;
      button.textContent =
        state.phase === "generating"
          ? "生成中…"
          : state.phase === "applying"
            ? "套用中…"
            : "生成配色";
      status.dataset.phase = state.phase;
      status.textContent = state.message;
    },
    {
      ...handlers,
      async applyImage(blob) {
        await handlers.applyImage(blob);
        if (disposed) return;
        const previousUrl = previewUrl;
        previewUrl = URL.createObjectURL(blob);
        previewImage.src = previewUrl;
        preview.hidden = false;
        if (previousUrl) URL.revokeObjectURL(previousUrl);
      },
    },
  );
  const onSubmit = (event: SubmitEvent) => {
    event.preventDefault();
    void controller.submit(input.value);
  };
  const onKeydown = (event: KeyboardEvent) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      void controller.submit(input.value);
    }
  };
  form.addEventListener("submit", onSubmit);
  input.addEventListener("keydown", onKeydown);
  return {
    dispose() {
      disposed = true;
      controller.dispose();
      form.removeEventListener("submit", onSubmit);
      input.removeEventListener("keydown", onKeydown);
      input.disabled = button.disabled = true;
      previewImage.removeAttribute("src");
      preview.hidden = true;
      if (previewUrl) URL.revokeObjectURL(previewUrl);
      previewUrl = undefined;
    },
  };
}
