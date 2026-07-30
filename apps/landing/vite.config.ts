import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Public landing site. Static build deployed to Vercel. It must NOT connect to
// the local daemon — it is marketing/onboarding only.
export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true },
});
