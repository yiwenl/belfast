export interface MotionProfile {
  type: "none" | "calm" | "wind" | "gust" | "swirl" | "rain";
  baseStrength: number;
  peakStrength: number;
  intervalSeconds: number;
  durationSeconds: number;
}

export async function analyzeMotion(text: string, signal: AbortSignal): Promise<MotionProfile> {
  const response = await fetch("/motion-api/analyze", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
    signal: AbortSignal.any([signal, AbortSignal.timeout(60000)]),
  });
  const data: unknown = await response.json();
  if (!response.ok) {
    const message =
      data && typeof data === "object" && "message" in data && typeof data.message === "string"
        ? data.message
        : "Motion service unavailable";
    throw new Error(`${message} (HTTP ${response.status})`);
  }
  if (!data || typeof data !== "object" || Array.isArray(data)) {
    throw new Error("Invalid motion response");
  }
  const value = data as Record<string, unknown>;
  const fields = ["baseStrength", "peakStrength", "intervalSeconds", "durationSeconds"];
  if (
    typeof value.type !== "string" ||
    !["none", "calm", "wind", "gust", "swirl", "rain"].includes(value.type) ||
    fields.some((field) => typeof value[field] !== "number" || !Number.isFinite(value[field]))
  ) {
    throw new Error("Invalid motion response");
  }
  // Rust validates numeric ranges and pulse relationships before changing the simulation.
  return {
    type: value.type as MotionProfile["type"],
    baseStrength: value.baseStrength as number,
    peakStrength: value.peakStrength as number,
    intervalSeconds: value.intervalSeconds as number,
    durationSeconds: value.durationSeconds as number,
  };
}
