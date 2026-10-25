// Keep these short and free of subjects or named dominant colors.
export const COLOR_PROMPT_SUFFIXES = [
  "bright midtones, crisp color contrast, subtle warm and cool accents",
  "luminous colors, clear tonal contrast, delicate complementary highlights",
  "well-lit colors, crisp contrast, subtle hue variation within the main palette",
  "bright natural light, distinct color separation, restrained secondary accents",
  "radiant midtones, rich local contrast, gentle reflected color",
  "clear bright colors, defined contrast, nuanced variations of the main colors",
] as const;

// ComfyUI CLIPTextEncode weights below 1 reduce this phrase's influence.
// https://docs.comfy.org/built-in-nodes/ClipTextEncode#prompt-weight-adjustment
export const COLOR_PROMPT_WEIGHT = 0.65;

export function enhanceColorPrompt(prompt: string): string {
  const suffix = COLOR_PROMPT_SUFFIXES[Math.floor(Math.random() * COLOR_PROMPT_SUFFIXES.length)];
  return `${prompt.trim()}, (${suffix}:${COLOR_PROMPT_WEIGHT})`;
}
