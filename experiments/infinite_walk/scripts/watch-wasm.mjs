import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
import { createBuildQueue, watchSources } from "./wasm-watch.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const withVite = process.argv.includes("--vite");
let child;
let server;
let listening;
let stopped = false;
let closeWatchers = () => {};
let builds = 0;

async function build() {
  const number = ++builds;
  console.log(`\n[wasm] Build #${number} (release)…`);
  const result = await new Promise((resolveBuild) => {
    // A separate process group lets Ctrl+C stop Cargo/wasm-bindgen as well.
    child = spawn(
      "wasm-pack",
      ["build", ".", "--target", "web", "--out-dir", "pkg", "--out-name", "experiment"],
      {
        cwd: root,
        stdio: "inherit",
        detached: process.platform !== "win32",
      },
    );
    child.once("error", (error) => {
      console.error(`[wasm] ${error.message}`);
      resolveBuild(1);
    });
    child.once("close", (code) => resolveBuild(code ?? 1));
  });
  child = undefined;
  if (stopped) return;
  if (result !== 0) {
    console.error(`[wasm] Build #${number} failed. Fix the source and save to retry.`);
    return;
  }
  console.log(`[wasm] Build #${number} complete.`);
  if (!withVite) return;
  if (!server) {
    const { createServer } = await import("vite");
    if (stopped) return;
    server = await createServer({
      root,
      // Ignore intermediate wasm-pack writes; reload only after a successful build.
      server: { watch: { ignored: ["**/pkg/**"] } },
    });
    if (stopped) {
      await server.close();
      return;
    }
    try {
      listening = server.listen();
      await listening;
      if (!stopped) server.printUrls();
    } catch (error) {
      console.error(error);
      await shutdown(1);
    }
  } else {
    server.moduleGraph.invalidateAll();
    server.ws.send({ type: "full-reload", path: "*" });
  }
}

const queue = createBuildQueue(build);

async function shutdown(exitCode) {
  if (stopped) return;
  stopped = true;
  queue.stop();
  closeWatchers();
  if (child?.pid) {
    const closed = new Promise((resolveClose) => child.once("close", resolveClose));
    try {
      if (process.platform === "win32") child.kill("SIGTERM");
      else process.kill(-child.pid, "SIGTERM");
    } catch (error) {
      if (error.code !== "ESRCH") console.error(error);
    }
    await closed;
  }
  // Vite can finish binding HTTP after close() if startup is still in progress.
  // Wait for that bind (or its failure) before closing the server.
  await listening?.catch(() => {});
  await server?.close();
  process.exitCode = exitCode;
}

process.once("SIGINT", () => void shutdown(130));
process.once("SIGTERM", () => void shutdown(143));
try {
  closeWatchers = watchSources(
    [resolve(root, "src"), resolve(root, "../../crates/belfast/src")],
    (file) => {
      console.log(`[wasm] Changed: ${file}`);
      queue.request();
    },
    (error) => {
      console.error(error);
      void shutdown(1);
    },
  );
  console.log(
    "[wasm] Watching experiment + Belfast src/**/*.rs and src/**/*.wgsl (Ctrl+C to stop).",
  );
  queue.request();
} catch (error) {
  console.error(error);
  await shutdown(1);
}
