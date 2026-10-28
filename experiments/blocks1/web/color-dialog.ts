import type { ExperimentApp } from "../pkg/experiment";
import { ParticleClient } from "./particle-client.mjs";
import { analyzeMotion } from "./motion-client";

const COLOR_PROMPTS = [
  "Gentle currents swirl slowly around a tropical coral reef, turquoise water, vivid orange and pink coral, deep cobalt blue channels, sunlit underwater photography",
  "Strong steady wind sweeps snow rapidly across Arctic mountains beneath northern lights, luminous emerald green and violet ribbons, deep indigo sky, soft pink snow",
  "Repeated strong gusts of wind send sand racing over desert dunes at sunset, burnt orange sand, golden ridges, deep plum shadows, a rose pink sky, cinematic landscape",
  "A gentle steady breeze carries autumn leaves slowly over a lake, crimson maple leaves, golden yellow birches, dark evergreen trees, warm afternoon sunlight",
  "A perfectly calm lavender field with no wind, flowers completely still at dawn, soft lilac flowers, rich purple rows, peach sunlight, pale mint green hills",
  "One sudden strong gust of wind blows snow quickly through a glacial ice cave, translucent electric blue ice, luminous cyan walls, deep navy crevices, bright white frost",
  "Heavy rain falls rapidly through a neon city street at night, hot pink and electric cyan lights reflected in wet pavement, violet shadows, warm amber windows",
  "Repeated gentle gusts of wind make wildflowers sway slowly, scarlet poppies, golden buttercups, cornflower blue petals, fresh lime green grass, bright summer sunlight",
  "A strong whirlpool swirls rapidly through a tropical lagoon seen from the air, jade green jungle, milky turquoise shallows, deep sapphire water, warm ivory sand",
  "Strong steady wind carries glowing embers rapidly over a volcanic landscape at dusk, fiery red and brilliant orange lava, golden sparks, charcoal rock, deep purple sky",
] as const;

export function createColorDialog(app: ExperimentApp) {
  const dialog = document.querySelector<HTMLDialogElement>("#color-dialog")!;
  const form = document.querySelector<HTMLFormElement>("#color-form")!;
  const prompt = document.querySelector<HTMLTextAreaElement>("#color-prompt")!;
  const submit = document.querySelector<HTMLButtonElement>("#color-submit")!;
  const random = document.querySelector<HTMLButtonElement>("#color-random")!;
  const close = document.querySelector<HTMLButtonElement>("#color-close")!;
  const status = document.querySelector<HTMLElement>("#color-status")!;
  const motionStatus = document.querySelector<HTMLElement>("#motion-status")!;
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

  async function generateColors(text: string) {
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
      if (!motionStatus.dataset.error) dialog.close();
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
    }
  }

  async function generateMotion(text: string) {
    try {
      const motion = await analyzeMotion(text, lifetime.signal);
      if (lifetime.signal.aborted) return;
      const applied = app.setMotion(
        motion.type,
        new Float32Array([
          motion.baseStrength,
          motion.peakStrength,
          motion.intervalSeconds,
          motion.durationSeconds,
        ]),
      );
      const timing =
        motion.durationSeconds === 0
          ? "steady"
          : motion.intervalSeconds > 0
            ? `${motion.durationSeconds}s bursts every ${motion.intervalSeconds}s`
            : `one ${motion.durationSeconds}s burst`;
      motionStatus.textContent = applied
        ? `Motion: ${motion.type} · ${motion.baseStrength.toFixed(2)}–${motion.peakStrength.toFixed(2)} · ${timing}`
        : "No supported motion found. Keeping the previous motion.";
    } catch (error) {
      if (lifetime.signal.aborted) return;
      const message = error instanceof Error ? error.message : String(error);
      motionStatus.textContent = `${message}. Your previous motion is unchanged.`;
      motionStatus.dataset.error = "true";
      open();
    }
  }

  async function generate(event: SubmitEvent) {
    event.preventDefault();
    const text = prompt.value.trim();
    if (busy || !text || lifetime.signal.aborted) return;
    if ([...text].length > 500) {
      status.textContent = "Keep the scene description within 500 characters.";
      status.dataset.error = "true";
      return;
    }
    busy = true;
    prompt.disabled = submit.disabled = random.disabled = true;
    submit.textContent = "Generating…";
    form.setAttribute("aria-busy", "true");
    status.textContent = "Connecting… Your particles will keep flowing.";
    motionStatus.textContent = "Analyzing motion…";
    delete status.dataset.error;
    delete motionStatus.dataset.error;
    try {
      // Apply each result independently; a failed service must not discard the other result.
      await Promise.all([generateColors(text), generateMotion(text)]);
    } finally {
      busy = false;
      if (!lifetime.signal.aborted) {
        prompt.disabled = submit.disabled = random.disabled = false;
        submit.textContent = "Generate scene";
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
