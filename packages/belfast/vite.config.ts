import { defineConfig } from "vite";
import dts from "vite-plugin-dts";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageDir = fileURLToPath(new URL(".", import.meta.url));
const distDir = resolve(packageDir, "../../dist");

export default defineConfig({
  build: {
    outDir: distDir,
    emptyOutDir: true,
    rollupOptions: {
      external: ["scheduling"],
    },
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      name: "belfast",
      formats: ["es", "cjs"],
      fileName: (format) => (format === "es" ? "belfast.js" : "belfast.cjs"),
    },
    sourcemap: true,
  },
  plugins: [
    dts({
      rollupTypes: true,
      tsconfigPath: "./tsconfig.json",
      beforeWriteFile(filePath, content) {
        if (!filePath.endsWith("index.d.ts") || content.includes("@webgpu/types")) {
          return;
        }
        return {
          filePath,
          content: `/// <reference types="@webgpu/types" />\n\n${content}`,
        };
      },
    }),
  ],
});
