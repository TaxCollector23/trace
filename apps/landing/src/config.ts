// Public links used across the landing site.
//
// Docs are hosted on GitHub Pages (built from /docs via apps/docs). Override
// with VITE_DOCS_URL at build time if the docs move.
export const DOCS_URL: string =
  import.meta.env.VITE_DOCS_URL || "https://taxcollector23.github.io/TraceGuard/";

// Back-compat alias (older imports referenced MINTLIFY_DOCS_URL).
export const MINTLIFY_DOCS_URL = DOCS_URL;

export const GITHUB_REPO = "https://github.com/TaxCollector23/TraceGuard";
export const RAW_BASE =
  "https://raw.githubusercontent.com/TaxCollector23/TraceGuard/main";
