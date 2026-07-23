import { Reveal } from "./components";

interface Connector {
  name: string;
  logo: JSX.Element;
}

/* ── Claude Code ──
   Anthropic starburst — elongated rounded petals radiating from center */
const ClaudeLogo = (
  <svg width="24" height="24" viewBox="0 0 100 100" fill="none">
    <g fill="#D97757">
      <rect x="46" y="4"  width="8" height="40" rx="4" transform="rotate(0 50 50)" />
      <rect x="46" y="8"  width="8" height="34" rx="4" transform="rotate(33 50 50)" />
      <rect x="46" y="4"  width="8" height="40" rx="4" transform="rotate(65 50 50)" />
      <rect x="46" y="10" width="8" height="32" rx="4" transform="rotate(98 50 50)" />
      <rect x="46" y="4"  width="8" height="40" rx="4" transform="rotate(130 50 50)" />
      <rect x="46" y="8"  width="8" height="34" rx="4" transform="rotate(163 50 50)" />
      <rect x="46" y="4"  width="8" height="40" rx="4" transform="rotate(196 50 50)" />
      <rect x="46" y="10" width="8" height="32" rx="4" transform="rotate(228 50 50)" />
      <rect x="46" y="4"  width="8" height="40" rx="4" transform="rotate(260 50 50)" />
      <rect x="46" y="8"  width="8" height="34" rx="4" transform="rotate(293 50 50)" />
      <rect x="46" y="4"  width="8" height="40" rx="4" transform="rotate(325 50 50)" />
    </g>
  </svg>
);

/* ── Codex CLI ──
   Purple gradient cloud/blob with white >_ terminal prompt */
const CodexLogo = (
  <svg width="24" height="24" viewBox="0 0 100 100" fill="none">
    <defs>
      <linearGradient id="codex-grad" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stopColor="#B8A9FF" />
        <stop offset="100%" stopColor="#7367F0" />
      </linearGradient>
    </defs>
    <path
      d="M50 8 C60 8 68 14 73 22 C80 18 90 24 92 36 C94 46 88 54 80 56
         C82 64 78 74 68 78 C60 80 54 76 50 72
         C46 76 40 80 32 78 C22 74 18 64 20 56
         C12 54 6 46 8 36 C10 24 20 18 27 22
         C32 14 40 8 50 8 Z"
      fill="url(#codex-grad)"
    />
    <path d="M35 48 L26 56 L35 64" stroke="#fff" strokeWidth="5.5" strokeLinecap="round" strokeLinejoin="round" fill="none" />
    <line x1="54" y1="64" x2="72" y2="64" stroke="#fff" strokeWidth="5.5" strokeLinecap="round" />
  </svg>
);

/* ── OpenCode ──
   Dark rounded rectangle with white top block and gray bottom block */
const OpenCodeLogo = (
  <svg width="24" height="24" viewBox="0 0 100 120" fill="none">
    <rect x="6" y="6" width="88" height="108" rx="8" fill="#1a1a1a" />
    <rect x="22" y="28" width="56" height="30" rx="2" fill="#ffffff" />
    <rect x="22" y="62" width="56" height="30" rx="2" fill="#b0b0b0" />
  </svg>
);

/* ── Cursor ──
   Official Cursor prism mark from cursor.com */
const CursorLogo = (
  <svg width="24" height="24" viewBox="0 0 512 512" fill="none">
    <rect width="512" height="512" rx="112" fill="#14120b" />
    <path
      d="M415 156.4L263.5 68.9c-4.9-2.8-10.9-2.8-15.7 0L96.3 156.4c-4.1 2.4-6.6 6.7-6.6 11.5v176.4c0 4.7 2.5 9.1 6.6 11.5l151.5 87.5c4.9 2.8 10.9 2.8 15.7 0L415 355.8c4.1-2.4 6.6-6.7 6.6-11.5V167.9c0-4.7-2.5-9.1-6.6-11.5zm-9.5 18.5L259.3 428.2c-1 1.7-3.6 1-3.6-1v-165.9c0-3.3-1.8-6.4-4.6-8L107.4 170.4c-1.7-1-1-3.6 1-3.6h292.5c4.2 0 6.8 4.5 4.7 8.1z"
      fill="#edecec"
    />
  </svg>
);

/* ── GitHub Copilot ──
   GitHub Octocat silhouette */
const CopilotLogo = (
  <svg width="24" height="24" viewBox="0 0 32 32" fill="none">
    <path
      fillRule="evenodd" clipRule="evenodd"
      d="M16 0C7.16 0 0 7.16 0 16c0 7.08 4.58 13.06 10.94 15.18.8.14 1.1-.34 1.1-.76 0-.38-.02-1.64-.02-2.98-4.02.74-5.06-1-5.38-1.9-.18-.46-.96-1.88-1.64-2.26-.56-.3-1.36-1.04-.02-1.06 1.26-.02 2.16 1.16 2.46 1.64 1.44 2.42 3.74 1.74 4.66 1.32.14-.96.56-1.66 1.02-2.06-3.56-.4-7.28-1.78-7.28-7.9 0-1.74.62-3.18 1.64-4.3-.16-.4-.72-2.04.16-4.24 0 0 1.34-.42 4.4 1.64 1.28-.36 2.64-.54 4-.54s2.72.18 4 .54c3.06-2.08 4.4-1.64 4.4-1.64.88 2.2.32 3.84.16 4.24 1.02 1.12 1.64 2.54 1.64 4.3 0 6.16-3.74 7.5-7.3 7.9.58.5 1.08 1.46 1.08 2.96 0 2.14-.02 3.86-.02 4.4 0 .42.3.92 1.1.76C27.42 29.06 32 23.06 32 16c0-8.84-7.16-16-16-16z"
      fill="#fafafa"
    />
  </svg>
);

const CONNECTORS: Connector[] = [
  { name: "Claude Code", logo: ClaudeLogo },
  { name: "Codex CLI", logo: CodexLogo },
  { name: "OpenCode", logo: OpenCodeLogo },
  { name: "Cursor", logo: CursorLogo },
  { name: "GitHub Copilot", logo: CopilotLogo },
];

export default function WorksEverywhere() {
  return (
    <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
      {CONNECTORS.map((c, i) => (
        <Reveal key={c.name} delay={i * 0.03}>
          <div className="flex items-center gap-3 py-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-surface-2">
              {c.logo}
            </div>
            <span className="text-sm font-medium text-text">{c.name}</span>
          </div>
        </Reveal>
      ))}
    </div>
  );
}
