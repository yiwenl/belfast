import type { ExperimentApp } from "../pkg/experiment";
import { ParticleClient } from "./particle-client.mjs";

const COLOR_PROMPTS = [
  "A tropical coral reef seen from above, turquoise water, vivid orange and pink coral, deep cobalt blue channels, sunlit underwater photography",
  "Northern lights flowing across an Arctic night sky, luminous emerald green and violet ribbons, deep indigo sky, soft pink glow on snowy mountains",
  "Rolling desert dunes at sunset, rich burnt orange sand, golden ridges, deep plum shadows, a rose pink sky, cinematic landscape photography",
  "An autumn forest reflected in a still lake, crimson maple leaves, golden yellow birches, dark evergreen trees, warm afternoon sunlight",
  "Endless lavender fields in Provence at dawn, soft lilac flowers, rich purple rows, peach sunlight, pale mint green hills, dreamy landscape photography",
  "Inside a glacial ice cave, translucent electric blue ice, luminous cyan walls, deep navy crevices, bright white frost, detailed nature photography",
  "A rainy neon city street at night, hot pink and electric cyan lights reflected in wet pavement, violet shadows, warm amber windows, cinematic photography",
  "A field of blooming wildflowers, scarlet poppies, golden buttercups, cornflower blue petals, fresh lime green grass, bright summer sunlight",
  "A tropical lagoon seen from the air, jade green jungle, milky turquoise shallows, deep sapphire water, warm ivory sand, bright natural sunlight",
  "Glowing lava flowing over black volcanic rock at dusk, fiery red and brilliant orange rivers, golden sparks, charcoal textures, deep purple sky",
] as const;

export function createColorDialog(app: ExperimentApp) {
  const dialog = document.querySelector<HTMLDialogElement>("#color-dialog")!;
  const form = document.querySelector<HTMLFormElement>("#color-form")!;
  const prompt = document.querySelector<HTMLTextAreaElement>("#color-prompt")!;
  const submit = document.querySelector<HTMLButtonElement>("#color-submit")!;
  const random = document.querySelector<HTMLButtonElement>("#color-random")!;
  const close = document.querySelector<HTMLButtonElement>("#color-close")!;
  const status = document.querySelector<HTMLElement>("#color-status")!;
  const preview = document.querySelector<HTMLImageElement>("#color-preview")!;
  const lifetime = new AbortController();
  const client = new ParticleClient({ baseUrl: new URL("/image-api", location.origin).href });
  let busy = false;
  let previewUrl: string | undefined;

  function open() {
    if (lifetime.signal.aborted || dialog.open) return;
    app.clearPointer();
    dialog.showModal();
    if (!busy) prompt.focus();
  }

  function generateRandom() {
    if (busy || lifetime.signal.aborted) return;
    prompt.value = COLOR_PROMPTS[Math.floor(Math.random() * COLOR_PROMPTS.length)];
    form.requestSubmit();
  }

  function waitForColorTransition(): Promise<void> {
    return new Promise((resolve, reject) => {
      let frame = 0;
      const onAbort = () => {
        cancelAnimationFrame(frame);
        reject(lifetime.signal.reason);
      };
      const tick = () => {
        if (lifetime.signal.aborted) {
          onAbort();
        } else if (app.isColorTransitioning()) {
          frame = requestAnimationFrame(tick);
        } else {
          lifetime.signal.removeEventListener("abort", onAbort);
          resolve();
        }
      };
      lifetime.signal.addEventListener("abort", onAbort, { once: true });
      tick();
    });
  }

  async function generate(event: SubmitEvent) {
    event.preventDefault();
    const text = prompt.value.trim();
    if (busy || !text || lifetime.signal.aborted) return;
    busy = true;
    prompt.disabled = submit.disabled = random.disabled = true;
    submit.textContent = "Generating…";
    form.setAttribute("aria-busy", "true");
    status.textContent = "Connecting… Your particles will keep flowing.";
    delete status.dataset.error;
    try {
      const response = await fetch("/image-api/workflow-api.json", {
        signal: AbortSignal.any([lifetime.signal, AbortSignal.timeout(15000)]),
      });
      if (!response.ok) throw new Error(`Image service unavailable (HTTP ${response.status})`);
      const workflow: unknown = await response.json();
      const result = await client.generate(workflow, text, {
        steps: 1,
        // Allow the model to load on the first request; do not generate a separate warmup.
        timeoutMs: 120000,
        signal: lifetime.signal,
        onEvent: (event) => {
          if (event.type === "execution_start" || event.type === "executing") {
            status.textContent = "Generating your color map…";
          }
        },
      });
      if (lifetime.signal.aborted) return;
      status.textContent = "Applying colors…";
      const bitmap = await createImageBitmap(result.blob);
      try {
        if (lifetime.signal.aborted) return;
        if (bitmap.width !== 512 || bitmap.height !== 512) {
          throw new Error(
            "The image service returned an unexpected image size; expected 512 × 512.",
          );
        }
        app.setColorMap(bitmap);
      } finally {
        bitmap.close();
      }
      if (previewUrl) URL.revokeObjectURL(previewUrl);
      previewUrl = URL.createObjectURL(result.blob);
      preview.src = previewUrl;
      preview.hidden = false;
      status.textContent = "Blending colors…";
      dialog.close();
      // Both palettes are in use until the fade ends; keep the next submission locked.
      await waitForColorTransition();
      if (lifetime.signal.aborted) return;
      status.textContent = "Colors updated.";
    } catch (error) {
      if (lifetime.signal.aborted) return;
      const message = error instanceof Error ? error.message : String(error);
      status.textContent = `${message}. Your previous colors are unchanged.`;
      status.dataset.error = "true";
      open();
    } finally {
      busy = false;
      if (!lifetime.signal.aborted) {
        prompt.disabled = submit.disabled = random.disabled = false;
        submit.textContent = "Generate colors";
        form.setAttribute("aria-busy", "false");
      }
    }
  }

  form.addEventListener("submit", generate, { signal: lifetime.signal });
  random.addEventListener("click", generateRandom, { signal: lifetime.signal });
  close.addEventListener("click", () => dialog.close(), { signal: lifetime.signal });
  return {
    open,
    generateRandom,
    dispose() {
      lifetime.abort();
      dialog.close();
      if (previewUrl) URL.revokeObjectURL(previewUrl);
      preview.removeAttribute("src");
      preview.hidden = true;
    },
  };
}
