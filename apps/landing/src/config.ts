// Public links used across the landing site.
//
// Docs are hosted on GitHub Pages (built from /docs via apps/docs). Override
// with VITE_DOCS_URL at build time if the docs move.
export const DOCS_URL: string =
  import.meta.env.VITE_DOCS_URL || "https://taxcollector23.github.io/trace/";

// Back-compat alias (older imports referenced MINTLIFY_DOCS_URL).
export const MINTLIFY_DOCS_URL = DOCS_URL;

export const GITHUB_REPO = "https://github.com/TaxCollector23/trace";
export const RAW_BASE =
  "https://raw.githubusercontent.com/TaxCollector23/trace/main";

// Where install.sh / install.ps1 are served from. We serve them from the
// landing site itself (see apps/landing/public/) rather than
// raw.githubusercontent so the pipe-to-shell one-liner shows a URL the
// visitor already trusts, same host as the landing they're reading.
// Override with VITE_LANDING_URL if this ever moves off the current
// Vercel project (e.g. a real trace.dev domain).
export const LANDING_URL: string =
  import.meta.env.VITE_LANDING_URL || "https://trace-landing-blue.vercel.app";
