import { defineConfig } from "vite";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sharedDir = path.dirname(fileURLToPath(import.meta.url));

export function createExampleConfig(exampleDir: string) {
  return defineConfig({
    root: exampleDir,
    resolve: {
      alias: {
        belfast: path.resolve(sharedDir, "../../packages/belfast/src/index.ts"),
      },
    },
    server: {
      open: true,
    },
  });
}
