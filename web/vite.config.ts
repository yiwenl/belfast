import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => ({
  server: {
    host: "localhost",
    port: 5173,
    strictPort: true,
    proxy: {
      "/comfy": {
        target: loadEnv(mode, process.cwd(), "COMFY_").COMFY_URL || "http://192.168.1.201:8189",
        changeOrigin: true,
        ws: true,
        rewrite: (path) => path.replace(/^\/comfy(?=\/|$)/, ""),
      },
    },
  },
  optimizeDeps: {
    exclude: ["@belfast/wasm"],
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        basic: "basic/index.html",
        template: "template/index.html",
        instancing: "instancing/index.html",
        particles: "particles/index.html",
        particlesImage: "particles-image/index.html",
      },
    },
  },
}));
