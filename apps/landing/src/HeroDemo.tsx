import { motion } from "framer-motion";

/**
 * The hero visual: an animated mockup of the Trace desktop app window,
 * showing agent sessions streaming in live. Replaces the old CLI-typing
 * terminal — the desktop app, not the command line, is the product story now.
 */
const SESSIONS = [
  { agent: "Claude Code", prompt: "fix the login bug and add tests", files: 7, cost: "$0.04", status: "good" as const },
  { agent: "Cursor", prompt: "rm -rf node_modules && npm install", files: 0, cost: "—", status: "bad" as const },
  { agent: "Codex CLI", prompt: "refactor auth middleware to use JWT", files: 3, cost: "$0.12", status: "good" as const },
  { agent: "OpenCode", prompt: "add pagination to /api/users", files: 5, cost: "$0.08", status: "warn" as const },
];

const container = {
  hidden: {},
  show: { transition: { staggerChildren: 0.12, delayChildren: 0.3 } },
};
const item = {
  hidden: { opacity: 0, y: 10 },
  show: { opacity: 1, y: 0, transition: { duration: 0.45, ease: [0.16, 1, 0.3, 1] as const } },
};

export default function HeroDemo() {
  return (
    <div className="relative">
      {/* ambient blue glow behind the window */}
      <div className="ambient-glow pointer-events-none absolute -inset-10 -z-10 rounded-full bg-brand/20 blur-3xl" />

      <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
        {/* title bar */}
        <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
          <span className="ml-2 text-xs font-medium text-text-dimmer">Trace — Session Timeline</span>
          <span className="ml-auto flex items-center gap-1.5 text-[11px] font-medium text-good">
            <span className="rec-dot h-1.5 w-1.5 rounded-full bg-good" />
            Recording
          </span>
        </div>

        <motion.div
          variants={container}
          initial="hidden"
          animate="show"
          className="grid grid-cols-[36px_1fr] gap-0"
        >
          {/* mini sidebar */}
          <div className="flex flex-col items-center gap-4 border-r border-border bg-surface py-5">
            {["dashboard", "timeline", "risk", "cost"].map((k) => (
              <span key={k} className="h-2 w-2 rounded-full bg-border-strong" />
            ))}
          </div>

          {/* session list */}
          <div className="space-y-2.5 p-4">
            {SESSIONS.map((s) => (
              <motion.div
                key={s.agent + s.prompt}
                variants={item}
                whileHover={{ y: -2 }}
                className="card-lift flex items-center justify-between gap-3 rounded-lg border border-border bg-white px-3.5 py-3"
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-text">{s.agent}</span>
                    <StatusDot status={s.status} />
                  </div>
                  <p className="mt-0.5 truncate font-mono text-xs text-text-dimmer">{s.prompt}</p>
                </div>
                <div className="shrink-0 text-right text-xs text-text-dimmer">
                  <div>{s.files} files</div>
                  <div className="font-mono">{s.cost}</div>
                </div>
              </motion.div>
            ))}
          </div>
        </motion.div>
      </div>
    </div>
  );
}

function StatusDot({ status }: { status: "good" | "warn" | "bad" }) {
  const cls = status === "good" ? "bg-good" : status === "warn" ? "bg-warn" : "bg-bad";
  return <span className={`h-1.5 w-1.5 rounded-full ${cls}`} />;
}
