import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

const proxy = {
  "/kinetic": {
    target: process.env.KINETIC_WS_URL || "ws://127.0.0.1:8787",
    ws: true,
    rewrite: () => "/",
  },
};

export default defineConfig(({ mode }) => ({
  base: "./",
  worker: { format: "es" },
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast firefly forest" })] : [],
  server: { host: "localhost", port: 5178, strictPort: true, proxy },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "localhost" : "localhost",
    port: mode === "lan" ? 5179 : 4175,
    strictPort: true,
    proxy,
  },
}));
