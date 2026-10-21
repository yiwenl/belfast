import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  base: "./",
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast experiment" })] : [],
  server: { host: "localhost", port: 5180, strictPort: true },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "localhost" : "localhost",
    port: mode === "lan" ? 5181 : 4176,
    strictPort: true,
  },
}));
