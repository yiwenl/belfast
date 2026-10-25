import init, { GaussianSorter } from "../pkg/experiment";
import type { SortReply, SortRequest } from "./sort-protocol";

// The project also compiles DOM code; keep Worker globals local to this module.
const scope = self as unknown as {
  onmessage: (event: MessageEvent<SortRequest>) => void;
  postMessage(message: SortReply, transfer?: Transferable[]): void;
};
let sorter: GaussianSorter | undefined;
let revision = -1;
let sortCount = 0;
let cullCount = 0;

scope.onmessage = async ({ data }) => {
  try {
    if (data.type === "init") {
      await init();
      sorter?.free();
      sorter = new GaussianSorter(data.positions);
      revision = -1;
      scope.postMessage({ type: "ready" });
      return;
    }
    if (!sorter) throw new Error("Gaussian sorter is not ready");
    if (data.revision !== revision) {
      // A new activation always captures a fresh order, even after rapid toggles.
      sorter.setFrozen(data.mode === "once");
      revision = data.revision;
      sortCount = 0;
      cullCount = 0;
    }
    const start = performance.now();
    const indices = sorter.sort(data.camera);
    const milliseconds = performance.now() - start;
    const didSort = sorter.didSort();
    sortCount += Number(didSort);
    cullCount += 1;
    scope.postMessage(
      { type: "sorted", indices, milliseconds, revision, didSort, sortCount, cullCount },
      [indices.buffer],
    );
  } catch (error) {
    scope.postMessage({ type: "error", message: String(error) });
  }
};
