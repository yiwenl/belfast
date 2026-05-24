import { defineConfig } from "vite";
import dts from "vite-plugin-dts";
import { resolve } from "node:path";

export default defineConfig({
  build: {
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
    }),
  ],
});
