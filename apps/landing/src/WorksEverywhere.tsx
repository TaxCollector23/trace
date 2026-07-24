import { motion } from "framer-motion";
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
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
      {CONNECTORS.map((c, i) => (
        <Reveal key={c.name} delay={i * 0.04}>
          <motion.div
            whileHover={{ y: -3 }}
            className="card-lift flex flex-col items-center gap-3 rounded-xl border border-border bg-white px-4 py-6 text-center shadow-sm"
          >
            <div className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-surface">
              <img src={c.logo} alt={`${c.name} logo`} className="h-full w-full object-cover" />
            </div>
            <span className="text-sm font-medium text-text">{c.name}</span>
          </motion.div>
        </Reveal>
      ))}
    </div>
  );
}
