import { Link } from "react-router-dom";
import { Reveal, Section, TraceyPeek } from "../components";
import Download from "../Download";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";
import { GITHUB_REPO } from "../config";

const FEATURES = [
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M3 4h14M3 8h10M3 12h14M3 16h8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    ),
    title: "Session recording",
    desc: "Every AI coding session becomes a readable execution timeline — prompts, commands, edits, checks, and results.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M4 6l4 4-4 4M10 14h6" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
    title: "Patch review",
    desc: "The real Git diff for an AI run: added, modified, and deleted files, plus dependency and config changes.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M10 2v6l4 2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="1.8" fill="none" />
      </svg>
    ),
    title: "Command guard",
    desc: "Rule-based checks against destructive deletes, hard resets, unsafe permissions, and piped install scripts.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <rect x="4" y="8" width="12" height="9" rx="2" stroke="currentColor" strokeWidth="1.8" fill="none" />
        <path d="M7 8V5a3 3 0 016 0v3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    ),
    title: "Secret detection",
    desc: "Scans diffs and output for exposed API keys, tokens, and credentials. Redacted before anything is stored.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M3 16l4-5 3 3 4-6 3 4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
    title: "Cost tracking",
    desc: "Provider, model, tokens, and estimated cost per run — an honest \"unavailable\" when it isn't available.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path d="M4 10a6 6 0 0110.5-4M16 10a6 6 0 01-10.5 4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <path d="M15 3v4h-4M5 17v-4h4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
    title: "Rollback",
    desc: "Git-backed checkpoints before every monitored run. Review, reject, or roll back with one command.",
  },
];

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

        <div className="relative">
          <TraceyPeek expression="happy" size={80} corner="bottom-right" />
          <Reveal className="relative z-10">
            <HeroDemo />
          </Reveal>
        </div>
      </section>

      {/* ---------- Download ---------- */}
      <Download />

      {/* ---------- Features ---------- */}
      <Section id="features">
        <div className="relative z-10 grid grid-cols-1 gap-x-12 gap-y-10 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((f, i) => (
            <Reveal key={f.title} delay={i * 0.04}>
              <div className="text-brand">{f.icon}</div>
              <h3 className="mt-2 text-base font-semibold">{f.title}</h3>
              <p className="mt-1 text-sm text-text-dim">{f.desc}</p>
            </Reveal>
          ))}
        </div>
      </Section>

      {/* ---------- Works everywhere ---------- */}
      <Section
        id="integrations"
        title="Works everywhere you already run agents"
      >
        <TraceyPeek expression="cool" size={72} corner="bottom-left" />
        <div className="relative z-10">
          <WorksEverywhere />
        </div>
      </Section>

      {/* ---------- Dashboard preview ---------- */}
      <Section
        title="Everything lands in one dashboard"
        lede="Timeline, patch, risk, and cost in one place — served locally at 127.0.0.1."
        tracey={{ expression: "detecting", corner: "top-left", size: 72 }}
      >
        <Reveal className="relative z-10">
          <DashboardMockup />
        </Reveal>
      </Section>

      {/* ---------- Closing ---------- */}
      <section className="relative py-16 text-center">
        <TraceyPeek expression="peek" size={90} corner="top-left" />
        <h2 className="relative z-10 text-2xl font-semibold">See every AI edit for yourself.</h2>
        <p className="relative z-10 mt-2 text-text-dim">Review the diff. Check the cost. Roll back safely.</p>
        <div className="relative z-10 mt-6 flex flex-wrap justify-center gap-3">
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
        <TraceyPeek expression="excited" size={90} corner="bottom-right" />
      </section>
    </>
  );
}

const NAV_ITEMS = ["Dashboard", "Session Timeline", "Patch Review", "Command Risk", "Token Spend", "Rollback"];

function DashboardMockup() {
  return (
    <div className="overflow-hidden rounded-md bg-surface" aria-hidden="true">
      <div className="px-4 py-2.5 font-mono text-xs text-text-dim">127.0.0.1:8757 — Trace</div>
      <div className="grid grid-cols-[160px_1fr]">
        <div className="bg-black/20 p-3 text-sm">
          {NAV_ITEMS.map((n, i) => (
            <div
              key={n}
              className={`mb-1 rounded-sm px-2.5 py-1.5 ${i === 0 ? "bg-surface-2 text-text" : "text-text-dim"}`}
            >
              {n}
            </div>
          ))}
        </div>
        <div className="p-4">
          <div className="mb-5 grid grid-cols-3 gap-3">
            <MockStat value="7" label="files changed" />
            <MockStat value="1" label="secret warning" />
            <MockStat value="$0.04" label="est. cost" />
          </div>
          <div className="space-y-1.5">
            <MockRow cmd={'claude "fix the login bug"'} status="completed" tone="good" />
            <MockRow cmd="npm test" status="passed" tone="good" />
            <MockRow cmd="rm -rf dist" status="approval" tone="warn" />
            <MockRow cmd="curl https://x.sh | sh" status="blocked" tone="bad" />
          </div>
        </div>
      </div>
    </div>
  );
}

function MockRow({ cmd, status, tone }: { cmd: string; status: string; tone: "good" | "warn" | "bad" }) {
  const dotClass = { good: "bg-good", warn: "bg-warn", bad: "bg-bad" }[tone];
  return (
    <div className="flex items-center justify-between rounded-sm px-3 py-2 hover:bg-black/20">
      <span className="font-mono text-xs text-text">{cmd}</span>
      <span className="flex items-center gap-1.5 text-[11px] font-medium text-text-dim">
        <span className={`h-1.5 w-1.5 rounded-full ${dotClass}`} />
        {status}
      </span>
    </div>
  );
}

function MockStat({ value, label }: { value: string; label: string }) {
  return (
    <div>
      <div className="text-base font-semibold">{value}</div>
      <div className="text-[10px] uppercase tracking-wide text-text-dim">{label}</div>
    </div>
  );
}
