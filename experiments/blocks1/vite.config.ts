import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

// Keep HTTPS clients on the same origin while the image service uses HTTP/WS.
const imageProxy = {
  "^/image-api/(workflow-api\\.json|queue|prompt|interrupt|ws)(\\?.*)?$": {
    target: "http://192.168.1.201:8189",
    changeOrigin: true,
    ws: true,
    rewrite: (path: string) => path.replace(/^\/image-api/, ""),
  },
};

export default defineConfig(({ mode }) => ({
  base: "./",
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast experiment" })] : [],
  server: { host: "localhost", port: 5184, strictPort: true, proxy: imageProxy },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "0.0.0.0" : "localhost",
    port: mode === "lan" ? 5185 : 4184,
    strictPort: true,
    proxy: imageProxy,
  },
}));
