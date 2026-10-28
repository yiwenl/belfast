import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  base: "./",
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast experiment" })] : [],
  server: { host: "localhost", port: 5184, strictPort: true },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "0.0.0.0" : "localhost",
    port: mode === "lan" ? 5185 : 4184,
    strictPort: true,
  },
}));
