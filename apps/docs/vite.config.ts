import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Docs site for GitHub Pages at https://taxcollector23.github.io/TraceGuard/.
// `base` must match the repo path. The docs markdown lives in /docs (single
// source of truth), so allow Vite to read one level above the app root.
export default defineConfig({
  base: "/TraceGuard/",
  plugins: [react()],
  server: { fs: { allow: ["../.."] } },
  build: { outDir: "dist", emptyOutDir: true },
});
