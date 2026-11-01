import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  base: "./",
  worker: { format: "es" },
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast courtyard experiment" })] : [],
  server: { host: "localhost", port: 5184, strictPort: true },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "localhost" : "localhost",
    port: mode === "lan" ? 5185 : 4178,
    strictPort: true,
  },
}));
