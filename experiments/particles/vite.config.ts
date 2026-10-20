import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  base: "./",
  worker: { format: "es" },
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast particle experiment" })] : [],
  server: { host: "localhost", port: 5176, strictPort: true },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "localhost" : "localhost",
    port: mode === "lan" ? 5177 : 4174,
    strictPort: true,
  },
}));
