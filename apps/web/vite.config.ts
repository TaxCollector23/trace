import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

/** Synthetic data exists only behind the Vite development server. It is never
 * copied into dist/ and therefore cannot become a production data source. */
function traceFixtureServer(): Plugin {
  return {
    name: "trace-dev-fixtures",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!req.url?.startsWith("/__trace-fixtures/")) return next();
        try {
          const fixtures = await server.ssrLoadModule("/src/devFixtures.ts");
          const url = new URL(req.url, "http://trace.local");
          const payload = url.pathname.endsWith("/connections")
            ? fixtures.createConnectionsFixture()
            : fixtures.createUsageFixture(
                url.searchParams.get("range") ?? "30d",
                url.searchParams.get("session_id") ?? undefined
              );
          res.statusCode = 200;
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify(payload));
        } catch (error) {
          res.statusCode = 500;
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify({ error: String(error) }));
        }
      });
    },
  };
}

// In development the dashboard runs on Vite's dev server and proxies /api to the
// local daemon. In production the daemon serves the built files from dist/ on
// the same origin, so the relative /api paths just work.
export default defineConfig({
  plugins: [react(), traceFixtureServer()],
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
