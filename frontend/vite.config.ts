import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": new URL("./src", import.meta.url).pathname },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": process.env.VITE_DEV_PROXY_TARGET ?? "http://127.0.0.1:3000",
      "/health": process.env.VITE_DEV_PROXY_TARGET ?? "http://127.0.0.1:3000",
    },
  },
})
