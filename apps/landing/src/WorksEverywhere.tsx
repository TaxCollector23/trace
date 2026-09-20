import { motion } from "framer-motion";
import { Reveal } from "./components";

interface Connector {
  name: string;
  logo: string;
  detail: string;
  protected: boolean;
}

const CONNECTORS: Connector[] = [
  { name: "Claude Code", logo: "/logos/claude.png", detail: "Stops risky commands", protected: true },
  { name: "Codex", logo: "/logos/codex.png", detail: "Stops risky commands", protected: true },
  { name: "OpenCode", logo: "/logos/opencode.png", detail: "Stops risky commands", protected: true },
  { name: "Cursor", logo: "/logos/cursor.png", detail: "Stops risky commands", protected: true },
  { name: "GitHub Copilot", logo: "/logos/copilot.png", detail: "Shows file changes", protected: false },
];

export default function WorksEverywhere() {
  return (
    <div>
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
        {CONNECTORS.map((c, i) => (
          <Reveal key={c.name} delay={i * 0.04} className="h-full">
            <motion.div
              whileHover={{ y: -3 }}
              className="card-lift flex h-full flex-col items-center justify-center gap-3 rounded-xl border border-border bg-white px-4 py-6 text-center shadow-sm"
            >
              {/* Fixed, identical box for every logo; object-contain so no logo is
                  cropped and each sits at the same scale. */}
              <div className="flex h-14 w-14 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-surface p-2">
                <img
                  src={c.logo}
                  alt={`${c.name} logo`}
                  className="h-full w-full object-contain"
                />
              </div>
              <span className="text-sm font-medium text-text">{c.name}</span>
              <span className={c.protected ? "text-xs text-emerald-700" : "text-xs text-text-muted"}>
                {c.detail}
              </span>
            </motion.div>
          </Reveal>
        ))}
      </div>
      <p className="mt-4 text-center text-xs text-text-muted">
        Windsurf can show activity and file changes, but it does not expose a hook that can stop a shell command.
      </p>
    </div>
  );
}
