import { Reveal } from "./components";

interface Connector {
  name: string;
  logo: string;
}

const CONNECTORS: Connector[] = [
  { name: "Claude Code", logo: "/logos/claude.png" },
  { name: "Codex CLI", logo: "/logos/codex.png" },
  { name: "OpenCode", logo: "/logos/opencode.png" },
  { name: "Cursor", logo: "/logos/cursor.png" },
  { name: "GitHub Copilot", logo: "/logos/copilot.png" },
];

export default function WorksEverywhere() {
  return (
    <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
      {CONNECTORS.map((c, i) => (
        <Reveal key={c.name} delay={i * 0.03}>
          <div className="flex items-center gap-3 py-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-surface-2">
              <img src={c.logo} alt={`${c.name} logo`} className="h-full w-full object-cover" />
            </div>
            <span className="text-sm font-medium text-text">{c.name}</span>
          </div>
        </Reveal>
      ))}
    </div>
  );
}
