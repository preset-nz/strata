import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import path from "path"
import { defineConfig } from "vite"

const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
    // The @preset.nz packages ship unbuilt TypeScript. Pin the shared runtime
    // to this app's copy; facets is in the list because two copies would mean
    // two registries.
    dedupe: [
      "react",
      "react-dom",
      "@tauri-apps/api",
      "@preset.nz/facets",
      "@preset.nz/app-kit",
      "@preset.nz/ux-kit",
    ],
  },
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  clearScreen: false,
  server: {
    port: 1450,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1451,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
})
