import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// In development the dashboard runs on Vite's dev server and proxies /api to the
// local daemon. In production the daemon serves the built files from dist/ on
// the same origin, so the relative /api paths just work.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:8757",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
