export type SortMode = "normal" | "once";

export type SortRequest =
  | { type: "init"; positions: Float32Array }
  | { type: "sort"; camera: Float32Array; mode: SortMode; revision: number };

export type SortReply =
  | { type: "ready" }
  | {
      type: "sorted";
      indices: Uint32Array;
      milliseconds: number;
      revision: number;
      didSort: boolean;
      sortCount: number;
      cullCount: number;
    }
  | { type: "error"; message: string };
