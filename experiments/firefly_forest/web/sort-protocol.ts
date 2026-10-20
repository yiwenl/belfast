export type SortRequest =
  | { type: "init"; positions: Float32Array }
  | { type: "sort"; camera: Float32Array };

export type SortReply =
  | { type: "ready" }
  | { type: "sorted"; indices: Uint32Array; milliseconds: number }
  | { type: "error"; message: string };
