import { Reveal } from "./components";

interface Connector {
  name: string;
  logo: JSX.Element;
}

const S = 22;

const ClaudeLogo = (
  <svg width={S} height={S} viewBox="0 0 64 64" fill="none">
    <path d="M37.5 13.4L24.3 41.8l-4.4-14.2L37.5 13.4z" fill="#D97757" />
    <path d="M39.6 41.8L26.5 13.4l17.6 14.2L39.6 41.8z" fill="#D97757" />
    <path d="M26.5 13.4h11L32 3 26.5 13.4z" fill="#D97757" />
    <path d="M24.3 41.8h15.3L32 52.2 24.3 41.8z" fill="#D97757" />
    <path d="M20 27.6l-5.6-4.4L24.3 41.8 20 27.6z" fill="#D97757" />
    <path d="M44 27.6l5.6-4.4L39.6 41.8 44 27.6z" fill="#D97757" />
  </svg>
);

const OpenAILogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <path
      d="M22.3 10.2c.4-1.5.1-3.1-.8-4.4-1.4-2.1-4-3.1-6.5-2.6C13.7 1.8 12 1 10.2 1 7.6 1 5.3 2.6 4.5 5c-1.6.3-2.9 1.3-3.7 2.6C-.1 9.8-.2 12.5 1 14.8c-.4 1.5-.1 3.1.8 4.4 1.4 2.1 4 3.1 6.5 2.6 1.3 1.4 3 2.2 4.8 2.2 2.6 0 4.9-1.6 5.7-4 1.6-.3 2.9-1.3 3.7-2.6.9-1.6 1-3.6.1-5.2h-.3z"
      fill="#10A37F"
    />
    <path
      d="M16.9 7.7l-5 2.9v5.7l-2-1.1V9.5l5-2.9 2 1.1zm-2 9.5v-2.3l5-2.9V9.7l-5 2.9-2-1.1 5-2.9 2 1.1v5.7l-5 2.8z"
      fill="#fff"
    />
  </svg>
);

const AiderLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <rect x="1" y="1" width="22" height="22" rx="5" fill="#14B8A6" />
    <text
      x="12"
      y="17.5"
      textAnchor="middle"
      fontSize="15"
      fontWeight="700"
      fill="#fff"
      fontFamily="system-ui, -apple-system, sans-serif"
    >
      a
    </text>
  </svg>
);

const OpenCodeLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <rect x="1" y="1" width="22" height="22" rx="5" fill="#3B82F6" />
    <path
      d="M9 8L5 12l4 4M15 8l4 4-4 4"
      stroke="#fff"
      strokeWidth="2.2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

const GeminiLogo = (
  <svg width={S} height={S} viewBox="0 0 28 28" fill="none">
    <path
      d="M14 0c0 0 2.5 7 6 10.5S28 14 28 14s-7 0-10.5 3.5S14 28 14 28s-2.5-7-6-10.5S0 14 0 14s7 0 10.5-3.5S14 0 14 0z"
      fill="#4285F4"
    />
  </svg>
);

const TerminalLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <rect x="1" y="2" width="22" height="20" rx="4" fill="#6B7280" />
    <path
      d="M7 9l3 3-3 3M13 15h4"
      stroke="#fff"
      strokeWidth="2.2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

const CONNECTORS: Connector[] = [
  { name: "Claude Code", logo: ClaudeLogo },
  { name: "Codex CLI", logo: OpenAILogo },
  { name: "Aider", logo: AiderLogo },
  { name: "OpenCode", logo: OpenCodeLogo },
  { name: "Gemini CLI", logo: GeminiLogo },
  { name: "Any terminal command", logo: TerminalLogo },
];

export default function WorksEverywhere() {
  return (
    <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
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
