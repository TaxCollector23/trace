import { Link } from "react-router-dom";
import { motion } from "framer-motion";
import { Reveal, Section } from "../components";
import Download from "../Download";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";
import HowToUse from "../HowToUse";
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
            className="mt-7 flex flex-wrap gap-3"
          >
            <motion.div whileHover={{ y: -2 }} whileTap={{ scale: 0.96 }}>
              <a
                href="#download"
                className="btn-pop block rounded-full bg-brand px-6 py-3 text-sm font-medium text-white shadow-glow"
              >
                Download for macOS
              </a>
            </motion.div>
            <motion.div whileHover={{ y: -2 }} whileTap={{ scale: 0.96 }}>
              <a
                href={GITHUB_REPO}
                target="_blank"
                rel="noreferrer"
                className="btn-pop block rounded-full border border-border bg-white px-6 py-3 text-sm font-medium text-text"
              >
                View on GitHub
              </a>
            </motion.div>
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

      {/* ---------- How to use each integration ---------- */}
      <Section id="how-to-use" title="How each one connects">
        <HowToUse />
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
          <div className="mt-7 flex flex-wrap justify-center gap-3">
            <motion.div whileHover={{ y: -2 }} whileTap={{ scale: 0.96 }}>
              <a href="#download" className="btn-pop block rounded-full bg-brand px-6 py-3 text-sm font-medium text-white shadow-glow">
                Download for macOS
              </a>
            </motion.div>
            <motion.div whileHover={{ y: -2 }} whileTap={{ scale: 0.96 }}>
              <Link to="/about" className="btn-pop block rounded-full border border-border bg-white px-6 py-3 text-sm font-medium text-text">
                About Trace
              </Link>
            </motion.div>
          </div>
        </Reveal>
      </section>
    </>
  );
}

/* ── Realistic dashboard mockup ── */

const SIDEBAR = [
  { name: "Dashboard", active: true },
  { name: "Session Timeline", active: false },
  { name: "Patch Review", active: false },
  { name: "Command Risk", active: false },
  { name: "Token Spend", active: false },
  { name: "Rollback Center", active: false },
  { name: "GitHub", active: false },
];

const SESSIONS = [
  { agent: "claude", prompt: "fix the login bug and add tests", files: 7, risk: "low", cost: "$0.04", status: "completed", time: "2m ago" },
  { agent: "codex", prompt: "refactor auth middleware to use JWT", files: 3, risk: "medium", cost: "$0.12", status: "completed", time: "18m ago" },
  { agent: "cursor", prompt: "add pagination to /api/users endpoint", files: 5, risk: "low", cost: "$0.08", status: "completed", time: "1h ago" },
  { agent: "claude", prompt: "rm -rf node_modules && npm install", files: 0, risk: "high", cost: "—", status: "blocked", time: "1h ago" },
];

function DashboardPreview() {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg" aria-hidden="true">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-border px-4 py-2.5">
        <div className="flex gap-1.5">
          <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        </div>
        <span className="ml-2 font-mono text-[11px] text-text-dimmer">Trace — Dashboard</span>
      </div>

      <div className="grid grid-cols-[180px_1fr]">
        {/* sidebar */}
        <div className="border-r border-border bg-surface p-3">
          <div className="mb-3 flex items-center gap-2 px-2 text-[13px] font-semibold text-text">
            <svg width="14" height="14" viewBox="0 0 194 200" fill="none">
              <rect x="31" y="32" width="132" height="30" rx="8" fill="#2f6fed" />
              <rect x="82" y="74" width="30" height="90" rx="8" fill="#2f6fed" />
            </svg>
            Trace
          </div>
          {SIDEBAR.map((item) => (
            <div
              key={item.name}
              className={`mb-0.5 rounded-lg px-2.5 py-1.5 text-[12px] ${
                item.active
                  ? "bg-brand-soft font-medium text-brand-dim"
                  : "text-text-dimmer hover:text-text-dim"
              }`}
            >
              {item.name}
            </div>
          ))}
        </div>

        {/* main content */}
        <div className="p-5">
          {/* KPI row */}
          <div className="mb-5 grid grid-cols-4 gap-3">
            <KPI value="4" label="Sessions today" />
            <KPI value="15" label="Files changed" />
            <KPI value="1" label="Blocked" color="text-bad" />
            <KPI value="$0.24" label="Total cost" />
          </div>

          {/* recent sessions */}
          <div className="mb-2 text-[11px] font-medium uppercase tracking-wider text-text-dimmer">Recent sessions</div>
          <div className="overflow-hidden rounded-lg border border-border">
            {/* header */}
            <div className="grid grid-cols-[1fr_60px_70px_60px_70px_70px] gap-2 bg-surface px-3 py-1.5 text-[10px] font-medium uppercase tracking-wider text-text-dimmer">
              <span>Prompt</span>
              <span>Agent</span>
              <span>Files</span>
              <span>Risk</span>
              <span>Cost</span>
              <span>Status</span>
            </div>
            {SESSIONS.map((s, i) => (
              <div
                key={i}
                className="grid grid-cols-[1fr_60px_70px_60px_70px_70px] gap-2 border-t border-border px-3 py-2 text-[12px]"
              >
                <span className="truncate font-mono text-[11px] text-text">{s.prompt}</span>
                <span className="text-text-dim">{s.agent}</span>
                <span className="text-text-dim">{s.files}</span>
                <RiskBadge level={s.risk} />
                <span className="font-mono text-text-dim">{s.cost}</span>
                <StatusBadge status={s.status} />
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
    <div className="rounded-lg border border-border bg-surface px-3 py-2.5">
      <div className={`text-lg font-semibold ${color || "text-text"}`}>{value}</div>
      <div className="text-[10px] uppercase tracking-wide text-text-dimmer">{label}</div>
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
  return <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium ${cls}`}>{level}</span>;
}

function StatusBadge({ status }: { status: string }) {
  const cls =
    status === "blocked"
      ? "text-bad"
      : status === "completed"
      ? "text-good"
      : "text-text-dim";
  return <span className={`text-[11px] font-medium ${cls}`}>{status}</span>;
}
