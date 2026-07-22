import { Reveal } from "./components";

interface Connector {
  name: string;
  logo: JSX.Element;
}

const S = 20;

const ClaudeLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <path d="M16.1 3.4L12 12l7.5-4.5c.7-.4.9-1.3.4-2l-.6-1c-.4-.7-1.3-.9-2-.5l-1.2.7v-.3z" fill="#D97757"/>
    <path d="M7.9 3.4L12 12 4.5 7.5c-.7-.4-.9-1.3-.4-2l.6-1c.4-.7 1.3-.9 2-.5l1.2.7v-.3z" fill="#D97757"/>
    <path d="M12 12l4.1 8.6 1.2-.7c.7-.4.9-1.3.5-2L12 12z" fill="#D97757"/>
    <path d="M12 12L7.9 20.6l-1.2-.7c-.7-.4-.9-1.3-.5-2L12 12z" fill="#D97757"/>
    <path d="M12 12v9c0 .8.7 1.5 1.5 1.5h1.2c.8 0 1.5-.7 1.5-1.5L12 12z" fill="#D97757"/>
    <path d="M12 12v9c0 .8-.7 1.5-1.5 1.5H9.3c-.8 0-1.5-.7-1.5-1.5L12 12z" fill="#D97757"/>
  </svg>
);

const OpenAILogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <path d="M22.3 10.2c.4-1.5.1-3.1-.8-4.4-1.4-2.1-4-3.1-6.5-2.6C13.7 1.8 12 1 10.2 1 7.6 1 5.3 2.6 4.5 5c-1.6.3-2.9 1.3-3.7 2.6C-.1 9.8-.2 12.5 1 14.8c-.4 1.5-.1 3.1.8 4.4 1.4 2.1 4 3.1 6.5 2.6 1.3 1.4 3 2.2 4.8 2.2 2.6 0 4.9-1.6 5.7-4 1.6-.3 2.9-1.3 3.7-2.6.9-1.6 1-3.6.1-5.2h-.3zM13.1 22c-1.2 0-2.3-.4-3.2-1.2l.2-.1 5.2-3c.3-.1.4-.4.4-.7v-7.3l2.2 1.3v6c0 2.8-2.2 5-5 5l.2-.1zM4.2 18.5c-.6-1-.8-2.2-.6-3.3l.2.1 5.2 3c.3.2.6.2.8 0l6.4-3.7v2.5l-5.4 3.1c-2.4 1.4-5.5.6-6.9-1.8l.3.1zM3 8.3c.6-1 1.5-1.8 2.6-2.2V13c0 .3.2.5.4.7l6.4 3.7L10.2 19 4.8 15.8c-2.4-1.3-3.3-4.4-2-6.8l.2-.7zM19.4 12.3l-6.4-3.7L15.2 7l5.4 3.1c2.4 1.4 3.3 4.4 2 6.8-.6 1-1.5 1.8-2.6 2.2v-6.9c0-.3-.2-.5-.4-.7l-.2-.2zM21.6 8c-.1-.1-.2-.1-.2-.1l-5.2-3c-.3-.2-.6-.2-.8 0L9 8.6V6.1l5.4-3.1c2.4-1.4 5.5-.6 6.9 1.8.6 1 .8 2.2.6 3.3l-.3-.1zM7.8 13.4L5.6 12V6c0-2.8 2.3-5.1 5.1-5 1.2 0 2.3.4 3.2 1.2l-.2.1-5.2 3c-.3.1-.4.4-.4.7l-.3 5.4z" fill="#10A37F"/>
  </svg>
);

const AiderLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <rect x="2" y="2" width="20" height="20" rx="4" fill="#14B8A6"/>
    <text x="12" y="17" textAnchor="middle" fontSize="14" fontWeight="700" fill="#fff" fontFamily="system-ui">a</text>
  </svg>
);

const OpenCodeLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <rect x="2" y="2" width="20" height="20" rx="4" fill="#3B82F6"/>
    <path d="M9 8L5 12l4 4M15 8l4 4-4 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);

const GeminiLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <path d="M12 2C12 2 14.5 7.5 17.5 10S24 12 24 12s-5.5 0-8.5 2-5.5 10-5.5 10-2.5-7.5-5.5-10S0 12 0 12s5.5 0 8.5-2S12 2 12 2z" fill="#4285F4"/>
  </svg>
);

const TerminalLogo = (
  <svg width={S} height={S} viewBox="0 0 24 24" fill="none">
    <rect x="2" y="3" width="20" height="18" rx="3" fill="#6B7280"/>
    <path d="M7 9l3 3-3 3M13 15h4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
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
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-surface-2">
              {c.logo}
            </div>
            <span className="text-sm font-medium text-text">{c.name}</span>
          </div>
        </Reveal>
      ))}
    </div>
  );
}
