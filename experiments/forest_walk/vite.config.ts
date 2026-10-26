import basicSsl from "@vitejs/plugin-basic-ssl";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  base: "./",
  plugins: mode === "lan" ? [basicSsl({ name: "Belfast forest walk" })] : [],
  server: { host: "127.0.0.1", port: 5182, strictPort: true },
  preview: {
    host: mode === "lan" ? process.env.LAN_HOST || "localhost" : "localhost",
    port: mode === "lan" ? 5183 : 4177,
    strictPort: true,
  },
}));
