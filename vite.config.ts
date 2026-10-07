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
    // The @preset.nz packages are path dependencies of unbuilt TypeScript.
    // Rollup resolves their imports from their real location, which has no
    // node_modules of its own, so pin the shared runtime to this app's copy.
    // facets is in the list because two copies would mean two registries;
    // app-kit (a path dependency with its own node_modules) per its README.
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
