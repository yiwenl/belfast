import { watch } from "node:fs";
import { extname, join } from "node:path";

/** Debounce editor saves and keep at most one build running and one pending. */
export function createBuildQueue(build, { debounceMs = 200, onError = console.error } = {}) {
  let timer;
  let running = false;
  let pending = false;
  let stopped = false;

  async function run() {
    timer = undefined;
    if (stopped || running || !pending) return;
    pending = false;
    running = true;
    try {
      await build();
    } catch (error) {
      onError(error);
    } finally {
      running = false;
      if (pending && !stopped) {
        clearTimeout(timer);
        timer = setTimeout(run, debounceMs);
      }
    }
  }

  return {
    request() {
      if (stopped) return;
      pending = true;
      clearTimeout(timer);
      timer = setTimeout(run, debounceMs);
    },
    stop() {
      stopped = true;
      pending = false;
      clearTimeout(timer);
    },
  };
}

/** Watch source directories only: pkg/dist/target cannot cause rebuild loops. */
export function watchSources(directories, onChange, onError = console.error) {
  const watchers = [];
  try {
    for (const directory of directories) {
      const watcher = watch(directory, { recursive: true }, (_event, name) => {
        // A missing filename means the OS cannot identify the changed file.
        if (name === null || [".rs", ".wgsl"].includes(extname(name))) {
          onChange(name === null ? directory : join(directory, name));
        }
      });
      watcher.on("error", onError);
      watchers.push(watcher);
    }
  } catch (error) {
    watchers.forEach((watcher) => watcher.close());
    throw error;
  }
  return () => watchers.forEach((watcher) => watcher.close());
}
