import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

// Keep HTTPS clients on the same origin while local services use HTTP/WS.
const apiProxy = {
  "^/kinetic-ws$": {
    target: process.env.KINETIC_WS_URL || "ws://127.0.0.1:8787",
    changeOrigin: true,
    ws: true,
    rewrite: () => "/",
  },
  "^/image-api/(workflow-api\\.json|queue|prompt|interrupt|ws)(\\?.*)?$": {
    target: "http://192.168.1.201:8189",
    changeOrigin: true,
    ws: true,
    rewrite: (path: string) => path.replace(/^\/image-api/, ""),
  },
  "^/motion-api/analyze$": {
    target: "http://192.168.1.201:8189",
    changeOrigin: true,
    // The service accepts only its own origin; expose this one route via Vite.
    headers: { Origin: "http://192.168.1.201:8189" },
    rewrite: (path: string) => path.replace(/^\/motion-api/, "/motion"),
  },
};

export default defineConfig(({ mode }) => ({
  base: "./",
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast experiment" })] : [],
  server: { host: "localhost", port: 5184, strictPort: true, proxy: apiProxy },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "0.0.0.0" : "localhost",
    port: mode === "lan" ? 5185 : 4184,
    strictPort: true,
    proxy: apiProxy,
  },
}));
