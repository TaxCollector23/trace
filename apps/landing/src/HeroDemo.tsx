import { motion } from "framer-motion";

/**
 * The hero visual: an animated mockup of the Trace desktop app reviewing a
 * single Claude Code session in detail — file list, real diff, cost, risk.
 * Deliberately one agent, one session, shown large and legible rather than
 * a cramped multi-row list.
 */
const FILES = [
  { path: "src/auth/login.ts", type: "modified" as const },
  { path: "src/auth/session.ts", type: "modified" as const },
  { path: "tests/auth/login.test.ts", type: "created" as const },
];

const container = {
  hidden: {},
  show: { transition: { staggerChildren: 0.1, delayChildren: 0.35 } },
};
const item = {
  hidden: { opacity: 0, y: 10 },
  show: { opacity: 1, y: 0, transition: { duration: 0.45, ease: [0.16, 1, 0.3, 1] as const } },
};

export default function HeroDemo() {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
        <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        <span className="ml-2 text-xs font-medium text-text-dimmer">Trace — Claude Code Session</span>
        <span className="ml-auto flex items-center gap-1.5 text-[11px] font-medium text-good">
          <span className="rec-dot h-1.5 w-1.5 rounded-full bg-good" />
          Recording
        </span>
      </div>

      <motion.div variants={container} initial="hidden" animate="show" className="p-5">
        {/* session header */}
        <motion.div variants={item} className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <img src="/logos/claude.png" alt="" className="h-7 w-7 rounded-md" />
            <div>
              <div className="text-sm font-semibold text-text">Claude Code</div>
              <div className="font-mono text-xs text-text-dimmer">fix the login bug and add tests</div>
            </div>
          </div>
          <span className="rounded-full bg-good-soft px-2.5 py-1 text-[11px] font-medium text-good">completed</span>
        </motion.div>

        {/* file list */}
        <motion.div variants={item} className="mt-4 space-y-1.5">
          {FILES.map((f) => (
            <div key={f.path} className="flex items-center gap-2 rounded-lg border border-border bg-surface px-3 py-2">
              <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${f.type === "created" ? "bg-good" : "bg-brand"}`} />
              <span className="truncate font-mono text-xs text-text-dim">{f.path}</span>
            </div>
          ))}
        </motion.div>

        {/* real diff excerpt */}
        <motion.div variants={item} className="mt-4 overflow-hidden rounded-lg border border-border bg-surface">
          <div className="border-b border-border px-3 py-1.5 font-mono text-[11px] text-text-dimmer">src/auth/login.ts</div>
          <pre className="overflow-x-auto p-3 font-mono text-[12.5px] leading-[1.7]">
            <span className="block text-text-dimmer">  return unauthorized();</span>
            <span className="block bg-bad-soft text-bad">- if (user.token = null) &#123;</span>
            <span className="block bg-good-soft text-good">+ if (user.token === null) &#123;</span>
            <span className="block text-text-dimmer">    return unauthorized();</span>
          </pre>
        </motion.div>

        {/* stat row */}
        <motion.div variants={item} className="mt-4 grid grid-cols-3 gap-2.5">
          <Stat label="Files changed" value="3" />
          <Stat label="Risk" value="Low" tone="good" />
          <Stat label="Cost" value="$0.02" />
        </motion.div>
      </motion.div>
    </div>
  );
}

function Stat({ label, value, tone }: { label: string; value: string; tone?: "good" }) {
  return (
    <div className="rounded-lg border border-border bg-white px-3 py-2.5 text-center">
      <div className={`font-serif text-xl ${tone === "good" ? "text-good" : "text-text"}`}>{value}</div>
      <div className="mt-0.5 text-[10px] uppercase tracking-wide text-text-dimmer">{label}</div>
    </div>
  );
}
