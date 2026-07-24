import { motion } from "framer-motion";
import { Reveal, Section, Button } from "../components";
import Download from "../Download";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";
import { GITHUB_REPO } from "../config";

const heroFade = {
  hidden: { opacity: 0, y: 14 },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.55, delay: 0.1 + i * 0.08, ease: [0.16, 1, 0.3, 1] as const },
  }),
};

export default function Home() {
  return (
    <>
      {/* ---------- Hero ---------- */}
      <section className="relative grid grid-cols-1 items-center gap-10 py-16 md:grid-cols-[1.05fr_1fr] md:py-24">
        <div>
          <motion.h1
            custom={0}
            initial="hidden"
            animate="show"
            variants={heroFade}
            className="font-serif text-4xl text-text md:text-5xl"
          >
            Know what your AI agent did before you ship it.
          </motion.h1>
          <motion.p
            custom={1}
            initial="hidden"
            animate="show"
            variants={heroFade}
            className="mt-5 max-w-[500px] text-lg leading-relaxed text-text-dim"
          >
            Trace is a desktop app that watches every file your AI agents touch —
            Claude Code, Codex, Cursor — and turns each session into a diff you can
            actually review, a cost you can see, and a checkpoint you can undo.
          </motion.p>
          <motion.div
            custom={2}
            initial="hidden"
            animate="show"
            variants={heroFade}
            className="mt-7 flex flex-wrap items-center gap-4"
          >
            <Button href="#download">Download for macOS</Button>
            <Button variant="secondary" href={GITHUB_REPO} target="_blank" rel="noreferrer">
              View on GitHub
            </Button>
          </motion.div>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 16, scale: 0.98 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.6, delay: 0.2, ease: [0.16, 1, 0.3, 1] }}
        >
          <HeroDemo />
        </motion.div>
      </section>

      {/* ---------- Download ---------- */}
      <Download />

      {/* ---------- Works everywhere ---------- */}
      <Section
        id="integrations"
        title="Works with the agents you already run"
        lede="Claude Code, Codex CLI, and OpenCode launch straight from the desktop app. Cursor and GitHub Copilot connect in."
      >
        <WorksEverywhere />
      </Section>

      {/* ---------- Dashboard ---------- */}
      <Section
        id="dashboard"
        title="Every session, laid out plainly"
        lede="Timeline, patch review, cost, risk, and rollback — one window, updated live."
      >
        <Reveal>
          <DashboardPreview />
        </Reveal>
      </Section>

      {/* ---------- Closing ---------- */}
      <section className="py-20 text-center">
        <Reveal>
          <h2 className="font-serif text-3xl text-text">See every AI edit for yourself.</h2>
          <p className="mt-3 text-text-dim">Review the diff. Check the cost. Roll back safely.</p>
          <div className="mt-7 flex flex-wrap items-center justify-center gap-4">
            <Button href="#download">Download for macOS</Button>
            <Button variant="secondary" to="/about">About Trace</Button>
          </div>
        </Reveal>
      </section>
    </>
  );
}

/* ── Realistic dashboard mockup — OpenCode, reviewing three real runs.
   A different agent from the hero (Claude Code) and the download section
   (Cursor), bigger and denser than the old cramped table. ── */

const SIDEBAR = [
  { name: "Dashboard", active: true },
  { name: "Session Timeline", active: false },
  { name: "Patch Review", active: false },
  { name: "Command Risk", active: false },
  { name: "Token Spend", active: false },
  { name: "Rollback Center", active: false },
];

const SESSIONS = [
  { prompt: "add pagination to /api/users endpoint", files: 5, risk: "low", cost: "$0.06", status: "completed", time: "4m ago" },
  { prompt: "migrate config loader to zod schemas", files: 8, risk: "low", cost: "$0.09", status: "completed", time: "31m ago" },
  { prompt: "curl https://get-tool.sh | sh", files: 0, risk: "high", cost: "—", status: "blocked", time: "1h ago" },
];

function DashboardPreview() {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg" aria-hidden="true">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-border px-5 py-3.5">
        <div className="flex gap-1.5">
          <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
          <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
          <span className="h-3 w-3 rounded-full bg-[#28c840]" />
        </div>
        <span className="ml-2 font-mono text-xs text-text-dimmer">Trace — OpenCode Sessions</span>
      </div>

      <div className="grid grid-cols-[210px_1fr]">
        {/* sidebar */}
        <div className="border-r border-border bg-surface p-4">
          <div className="mb-4 flex items-center gap-2 px-2 text-[15px] font-semibold text-text">
            <svg width="16" height="16" viewBox="0 0 194 200" fill="none">
              <rect x="31" y="32" width="132" height="30" rx="8" fill="#2f6fed" />
              <rect x="82" y="74" width="30" height="90" rx="8" fill="#2f6fed" />
            </svg>
            Trace
          </div>
          {SIDEBAR.map((item) => (
            <div
              key={item.name}
              className={`mb-1 rounded-lg px-3 py-2 text-[13.5px] ${
                item.active
                  ? "bg-brand-soft font-medium text-brand-dim"
                  : "text-text-dimmer"
              }`}
            >
              {item.name}
            </div>
          ))}
        </div>

        {/* main content */}
        <div className="p-6">
          {/* KPI row */}
          <div className="mb-6 grid grid-cols-4 gap-3">
            <KPI value="3" label="Sessions today" />
            <KPI value="13" label="Files changed" />
            <KPI value="1" label="Blocked" color="text-bad" />
            <KPI value="$0.15" label="Total cost" />
          </div>

          {/* recent sessions */}
          <div className="mb-3 flex items-center gap-2 text-[12px] font-medium uppercase tracking-wider text-text-dimmer">
            <img src="/logos/opencode.png" alt="" className="h-4 w-4 rounded-sm" />
            OpenCode — recent sessions
          </div>
          <div className="space-y-2.5">
            {SESSIONS.map((s, i) => (
              <div
                key={i}
                className="flex items-center justify-between gap-4 rounded-lg border border-border px-4 py-3"
              >
                <div className="min-w-0">
                  <span className="truncate font-mono text-[13px] text-text">{s.prompt}</span>
                  <div className="mt-1 text-[11px] text-text-dimmer">{s.time} · {s.files} files</div>
                </div>
                <div className="flex shrink-0 items-center gap-3">
                  <RiskBadge level={s.risk} />
                  <span className="w-14 text-right font-mono text-[12px] text-text-dim">{s.cost}</span>
                  <StatusBadge status={s.status} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function KPI({ value, label, color }: { value: string; label: string; color?: string }) {
  return (
    <div className="rounded-lg border border-border bg-surface px-4 py-3">
      <div className={`font-serif text-2xl ${color || "text-text"}`}>{value}</div>
      <div className="mt-0.5 text-[10.5px] uppercase tracking-wide text-text-dimmer">{label}</div>
    </div>
  );
}

function RiskBadge({ level }: { level: string }) {
  const cls =
    level === "high"
      ? "bg-bad-soft text-bad"
      : level === "medium"
      ? "bg-warn-soft text-warn"
      : "bg-good-soft text-good";
  return <span className={`inline-flex items-center rounded-full px-2.5 py-1 text-[11px] font-medium ${cls}`}>{level}</span>;
}

function StatusBadge({ status }: { status: string }) {
  const cls =
    status === "blocked"
      ? "text-bad"
      : status === "completed"
      ? "text-good"
      : "text-text-dim";
  return <span className={`w-20 text-right text-[12px] font-medium ${cls}`}>{status}</span>;
}
