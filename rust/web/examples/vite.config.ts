import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: {
    exclude: ["belfast-wasm"],
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        basic: "basic/index.html",
        template: "template/index.html",
      },
    },
  },
});
