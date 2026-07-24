import { motion } from "framer-motion";
import { Reveal } from "./components";

interface Guide {
  name: string;
  logo: string;
  kind: "Launches for you" | "Connects in";
  copy: string;
}

const GUIDES: Guide[] = [
  {
    name: "Claude Code",
    logo: "/logos/claude.png",
    kind: "Launches for you",
    copy: "Pick a project in the desktop app and start a session — Trace launches Claude Code, watches every file edit and command, and tracks cost as it goes.",
  },
  {
    name: "Codex CLI",
    logo: "/logos/codex.png",
    kind: "Launches for you",
    copy: "Same flow as Claude Code. Choose Codex from the New Session screen and Trace records the whole run automatically.",
  },
  {
    name: "Cursor",
    logo: "/logos/cursor.png",
    kind: "Connects in",
    copy: "Cursor connects to Trace as an MCP server — the desktop app gives you the config to paste into Cursor's settings, then every edit shows up live.",
  },
  {
    name: "OpenCode",
    logo: "/logos/opencode.png",
    kind: "Launches for you",
    copy: "Launch OpenCode from the desktop app the same way as any other agent — recorded, reviewed, and cost-tracked from the first keystroke.",
  },
  {
    name: "GitHub Copilot",
    logo: "/logos/copilot.png",
    kind: "Connects in",
    copy: "A companion VS Code extension bridges Copilot Chat to your desktop app — file changes and risky edits are flagged as you go.",
  },
];

export default function HowToUse() {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
      {GUIDES.map((g, i) => (
        <Reveal key={g.name} delay={i * 0.04}>
          <motion.div
            whileHover={{ y: -2 }}
            className="card-lift h-full rounded-xl border border-border bg-white p-5 shadow-sm"
          >
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-surface">
                <img src={g.logo} alt={`${g.name} logo`} className="h-full w-full object-cover" />
              </div>
              <span className="text-sm font-medium text-text">{g.name}</span>
              <span className="ml-auto rounded-full bg-brand-soft px-2.5 py-1 text-[11px] font-medium text-brand-dim">
                {g.kind}
              </span>
            </div>
            <p className="mt-4 text-sm leading-relaxed text-text-dim">{g.copy}</p>
          </motion.div>
        </Reveal>
      ))}
    </div>
  );
}
