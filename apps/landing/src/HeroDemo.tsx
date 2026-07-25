import { useState } from "react";
import { motion } from "framer-motion";

/**
 * The hero visual: an animated, clickable mockup of the Trace desktop app
 * reviewing a single Claude Code session — file list, real diff, cost,
 * risk. Click a file to see its diff; nothing here calls a real API, it's
 * illustrative, but it responds instead of sitting static.
 */
const FILES = [
  {
    path: "src/auth/login.ts",
    type: "modified" as const,
    diff: [
      { text: "  return unauthorized();", tone: "dim" as const },
      { text: "- if (user.token = null) {", tone: "del" as const },
      { text: "+ if (user.token === null) {", tone: "add" as const },
      { text: "    return unauthorized();", tone: "dim" as const },
    ],
  },
  {
    path: "src/auth/session.ts",
    type: "modified" as const,
    diff: [
      { text: "- const ttl = 3600;", tone: "del" as const },
      { text: "+ const ttl = 60 * 60 * 8;", tone: "add" as const },
      { text: "  session.expiresAt = now + ttl;", tone: "dim" as const },
    ],
  },
  {
    path: "tests/auth/login.test.ts",
    type: "created" as const,
    diff: [
      { text: "+ it('rejects a null token', () => {", tone: "add" as const },
      { text: "+   expect(login(null)).toBe(false);", tone: "add" as const },
      { text: "+ });", tone: "add" as const },
    ],
  },
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
  const [active, setActive] = useState(0);
  const file = FILES[active];

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
        <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        <span className="ml-2 text-xs font-medium text-brand">Trace — Claude Code Session</span>
        <span className="ml-auto flex items-center gap-1.5 text-[11px] font-medium text-good">
          <span className="rec-dot h-1.5 w-1.5 rounded-full bg-good" />
          Recording
        </span>
      </div>

      <motion.div variants={container} initial="hidden" animate="show" className="p-5">
        {/* session header */}
        <motion.div variants={item} className="flex items-center justify-between gap-3">
          <div>
            <div className="text-sm font-semibold text-text">Claude Code</div>
            <div className="font-mono text-xs text-text-dim">fix the login bug and add tests</div>
          </div>
          <span className="rounded-full bg-good-soft px-2.5 py-1 text-[11px] font-medium text-good">completed</span>
        </motion.div>

        {/* file list — click to preview that file's diff below */}
        <motion.div variants={item} className="mt-4 space-y-1.5">
          {FILES.map((f, i) => (
            <button
              key={f.path}
              onClick={() => setActive(i)}
              className={`flex w-full items-center gap-2 rounded-lg border px-3 py-2 text-left transition-colors ${
                i === active ? "border-brand bg-brand-soft" : "border-border bg-surface hover:bg-surface-2"
              }`}
            >
              <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${f.type === "created" ? "bg-good" : "bg-brand"}`} />
              <span className="truncate font-mono text-xs text-text-dim">{f.path}</span>
            </button>
          ))}
        </motion.div>

        {/* diff for the selected file */}
        <motion.div variants={item} className="mt-4 overflow-hidden rounded-lg border border-border bg-surface">
          <div className="border-b border-border px-3 py-1.5 font-mono text-[11px] text-text-dim">{file.path}</div>
          <pre className="overflow-x-auto p-3 font-mono text-[12.5px] leading-[1.7]">
            {file.diff.map((line, i) => (
              <span
                key={i}
                className={`block ${
                  line.tone === "add"
                    ? "bg-good-soft text-good"
                    : line.tone === "del"
                    ? "bg-bad-soft text-bad"
                    : "text-text-dim"
                }`}
              >
                {line.text}
              </span>
            ))}
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
      <div className={`text-base font-semibold ${tone === "good" ? "text-good" : "text-text"}`}>{value}</div>
      <div className="mt-0.5 text-[10px] uppercase tracking-wide text-text-dim">{label}</div>
    </div>
  );
}
