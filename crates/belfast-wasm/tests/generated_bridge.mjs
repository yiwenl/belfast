import { readdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const testDirectory = dirname(fileURLToPath(import.meta.url));
const packageDirectory = resolve(testDirectory, "../../../packages/belfast-wasm/dist");
const wasm = await import(pathToFileURL(resolve(packageDirectory, "index.js")));
const snippetRoot = resolve(packageDirectory, "snippets");
const snippetDirectories = (await readdir(snippetRoot, { withFileTypes: true })).filter((entry) =>
  entry.isDirectory(),
);

if (snippetDirectories.length !== 1) {
  throw new Error(`expected one generated bridge snippet, found ${snippetDirectories.length}`);
}

const bridge = await import(
  pathToFileURL(resolve(snippetRoot, snippetDirectories[0].name, "inline0.js"))
);

function expectRejectedBeforeMethod(ExpectedClass, OtherClass, label) {
  let forgedCalled = false;
  const forged = {
    __wbg_ptr: 7,
    __frameHandle() {
      forgedCalled = true;
    },
  };
  if (ExpectedClass.__unwrap(bridge.borrowWasmClass(forged)) !== 0 || forgedCalled) {
    throw new Error(`${label} accepted an ordinary forged wrapper`);
  }

  const wrongClass = OtherClass.__wrap(9);
  if (ExpectedClass.__unwrap(bridge.borrowWasmClass(wrongClass)) !== 0) {
    throw new Error(`${label} accepted a wrong-class wrapper`);
  }
  wrongClass.__destroy_into_raw();

  let freedCalled = false;
  const freed = ExpectedClass.__wrap(0);
  freed.__frameHandle = () => {
    freedCalled = true;
  };
  if (ExpectedClass.__unwrap(bridge.borrowWasmClass(freed)) !== 0 || freedCalled) {
    throw new Error(`${label} invoked a method on a freed wrapper`);
  }
  freed.__destroy_into_raw();
}

expectRejectedBeforeMethod(wasm.RenderTarget, wasm.BindGroup, "RenderTarget");
expectRejectedBeforeMethod(wasm.BindGroup, wasm.RenderTarget, "BindGroup");

console.log("generated bridge guards: 6 passed, 0 failed");
