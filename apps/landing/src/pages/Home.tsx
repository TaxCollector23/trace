import { Link } from "react-router-dom";
import { Reveal, Section } from "../components";
import Download from "../Download";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";
import HowToUse from "../HowToUse";
import { GITHUB_REPO } from "../config";

export default function Home() {
  return (
    <>
      {/* ---------- Hero ---------- */}
      <section className="relative grid grid-cols-1 items-center gap-10 py-16 md:grid-cols-[1.05fr_1fr] md:py-24">
        <div>
          <h1 className="text-3xl font-semibold md:text-4xl">
            Know what your AI agent did before you ship it.
          </h1>
          <p className="mt-4 max-w-[520px] text-lg text-text-dim">
            Trace records file changes, shell commands, risky actions, token cost,
            and rollback checkpoints — all from a local dashboard on your machine.
          </p>
          <div className="mt-6 flex flex-wrap gap-3">
            <a
              href="#download"
              className="rounded bg-brand px-5 py-2.5 text-sm font-medium text-white hover:bg-brand-dim"
            >
              Install the CLI
            </a>
            <a
              href={`${GITHUB_REPO}`}
              target="_blank"
              rel="noreferrer"
              className="rounded bg-surface px-5 py-2.5 text-sm font-medium text-text hover:bg-surface-2"
            >
              View on GitHub
            </a>
          </div>
        </div>

        <Reveal>
          <HeroDemo />
        </Reveal>
      </section>

      {/* ---------- Download ---------- */}
      <Download />

      {/* ---------- Works everywhere ---------- */}
      <Section
        id="integrations"
        title="Works everywhere you already run agents"
      >
        <WorksEverywhere />
      </Section>

      {/* ---------- How to use each integration ---------- */}
      <Section
        id="how-to-use"
        title="How to connect each one"
        lede="CLI agents are wrapped directly. GUI tools connect through MCP or a bridge extension instead."
      >
        <HowToUse />
      </Section>

      {/* ---------- Dashboard ---------- */}
      <Section id="dashboard">
        <Reveal>
          <DashboardPreview />
        </Reveal>
      </Section>

      {/* ---------- Closing ---------- */}
      <section className="py-16 text-center">
        <h2 className="text-2xl font-semibold">See every AI edit for yourself.</h2>
        <p className="mt-2 text-text-dim">Review the diff. Check the cost. Roll back safely.</p>
        <div className="mt-6 flex flex-wrap justify-center gap-3">
          <a href="#download" className="rounded bg-brand px-5 py-2.5 text-sm font-medium text-white hover:bg-brand-dim">
            Install the CLI
          </a>
          <a href={GITHUB_REPO} target="_blank" rel="noreferrer" className="rounded bg-surface px-5 py-2.5 text-sm font-medium text-text hover:bg-surface-2">
            View on GitHub
          </a>
          <Link to="/about" className="rounded bg-surface px-5 py-2.5 text-sm font-medium text-text hover:bg-surface-2">
            About
          </Link>
        </div>
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
    <div className="overflow-hidden rounded-lg border border-white/[0.06] bg-[#0c0c10]" aria-hidden="true">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-white/[0.06] px-4 py-2">
        <div className="flex gap-1.5">
          <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        </div>
        <span className="ml-2 font-mono text-[11px] text-text-dimmer">127.0.0.1:8757 — Trace Dashboard</span>
      </div>

      <div className="grid grid-cols-[180px_1fr]">
        {/* sidebar */}
        <div className="border-r border-white/[0.06] bg-[#0a0a0e] p-3">
          <div className="mb-3 flex items-center gap-2 px-2 text-[13px] font-semibold text-text">
            <svg width="14" height="14" viewBox="0 0 194 200" fill="none">
              <rect x="31" y="32" width="132" height="30" rx="8" fill="#5b93f5" />
              <rect x="82" y="74" width="30" height="90" rx="8" fill="#5b93f5" />
            </svg>
            Trace
          </div>
          {SIDEBAR.map((item) => (
            <div
              key={item.name}
              className={`mb-0.5 rounded px-2.5 py-1.5 text-[12px] ${
                item.active
                  ? "bg-white/[0.06] font-medium text-text"
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
            <KPI value="1" label="Blocked" color="text-red-400" />
            <KPI value="$0.24" label="Total cost" />
          </div>

          {/* recent sessions */}
          <div className="text-[11px] font-medium uppercase tracking-wider text-text-dimmer mb-2">Recent sessions</div>
          <div className="rounded-md border border-white/[0.06] overflow-hidden">
            {/* header */}
            <div className="grid grid-cols-[1fr_60px_70px_60px_70px_70px] gap-2 bg-white/[0.02] px-3 py-1.5 text-[10px] font-medium uppercase tracking-wider text-text-dimmer">
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
                className="grid grid-cols-[1fr_60px_70px_60px_70px_70px] gap-2 border-t border-white/[0.04] px-3 py-2 text-[12px]"
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
    <div className="rounded-md border border-white/[0.06] bg-white/[0.02] px-3 py-2.5">
      <div className={`text-lg font-semibold ${color || "text-text"}`}>{value}</div>
      <div className="text-[10px] uppercase tracking-wide text-text-dimmer">{label}</div>
    </div>
  );
}

function RiskBadge({ level }: { level: string }) {
  const cls =
    level === "high"
      ? "bg-red-500/15 text-red-400"
      : level === "medium"
      ? "bg-yellow-500/15 text-yellow-400"
      : "bg-green-500/15 text-green-400";
  return <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium ${cls}`}>{level}</span>;
}

function StatusBadge({ status }: { status: string }) {
  const cls =
    status === "blocked"
      ? "text-red-400"
      : status === "completed"
      ? "text-green-400"
      : "text-text-dim";
  return <span className={`text-[11px] font-medium ${cls}`}>{status}</span>;
}
