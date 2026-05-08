import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri devUrl と一致 (5176)
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5176,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    minify: "esbuild",
    sourcemap: false,
  },
});
