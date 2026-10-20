import init, { GaussianSorter } from "../pkg/experiment";
import type { SortReply, SortRequest } from "./sort-protocol";

const scope = self as unknown as {
  onmessage: (event: MessageEvent<SortRequest>) => void;
  postMessage(message: SortReply, transfer?: Transferable[]): void;
};
let sorter: GaussianSorter | undefined;

scope.onmessage = async ({ data }) => {
  try {
    if (data.type === "init") {
      await init();
      sorter?.free();
      sorter = new GaussianSorter(data.positions);
      scope.postMessage({ type: "ready" });
      return;
    }
    if (!sorter) throw new Error("Gaussian sorter is not ready");
    const start = performance.now();
    const indices = sorter.sort(data.camera);
    scope.postMessage({ type: "sorted", indices, milliseconds: performance.now() - start }, [
      indices.buffer,
    ]);
  } catch (error) {
    scope.postMessage({ type: "error", message: String(error) });
  }
};
