import { useState, type ReactNode } from "react";
import { motion } from "framer-motion";
import { Reveal, Section, Button } from "../components";
import { DOCS_URL } from "../config";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";
import Terminal, { type TermLine } from "../Terminal";

const heroFade = {
  hidden: { opacity: 0, y: 14 },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.55, delay: 0.1 + i * 0.08, ease: [0.16, 1, 0.3, 1] as const },
  }),
};

// The real output of `trc integrations install all`, replayed line by line in
// the wire-up terminal. Agent names are shown as proper nouns.
const WIRE_UP_OUTPUT: TermLine[] = [
  { text: "Trace is watching", cls: "text-white/55" },
  { text: "  ✓ Claude Code", cls: "text-emerald-400/90" },
  { text: "  ✓ Codex CLI", cls: "text-emerald-400/90" },
  { text: "  ✓ Cursor", cls: "text-emerald-400/90" },
  { text: "  ✓ Windsurf", cls: "text-emerald-400/90" },
  { text: "  ✓ OpenCode", cls: "text-emerald-400/90" },
];

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
            className="mt-5 max-w-[520px] text-lg leading-relaxed text-text-dim"
          >
            Your AI can move fast. Trace gives you the seatbelt: a clear record of
            every command, file change, cost, and decision, with a local dashboard
            that lets you stop risky work and undo a run when you need to.
          </motion.p>
          <motion.div
            custom={2}
            initial="hidden"
            animate="show"
            variants={heroFade}
            className="mt-10 flex flex-wrap items-center gap-5"
          >
            <Button href="#install">Download the CLI</Button>
            <Button variant="secondary" href={DOCS_URL} target="_blank" rel="noreferrer">
              Read the documentation
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

      {/* ---------- Install (right below the hero) ---------- */}
      <section id="install" className="scroll-mt-24 py-6">
        <Terminal
          label="trace - install"
          command="npm i -g trace-agent-cli"
          copyText="npm i -g trace-agent-cli"
        />
      </section>

      {/* ---------- Works everywhere ---------- */}
      <Section
        id="integrations"
        title="Works with the agents you already run"
        lede="Connect the coding agents you already use. Trace shows what each one did, what it changed, and whether it tried something risky."
      >
        <WorksEverywhere />
      </Section>

      {/* ---------- One command wires up every agent ---------- */}
      <Section
        id="wire-up"
        title="One command wires up every agent you use"
        lede="Run one command. Trace adds its connection, keeps a backup of settings it touches, and starts the local dashboard for you."
      >
        <Terminal
          label="trace - integrations"
          command="trc integrations install all"
          output={WIRE_UP_OUTPUT}
        />
      </Section>

      {/* ---------- Trace Ratification ---------- */}
      <Section
        id="ratification"
        title="Check a pull request before it lands"
        lede="Review a pull request with the same checks that protect local work. See secrets, risky changes, and missing tests before they reach your branch."
      >
        <Reveal>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
              <div className="font-serif text-lg text-text">The same checks everywhere</div>
              <p className="mt-2 text-sm leading-relaxed text-text-dim">
                The checks that protect an agent run can also review a local change, a build, or a
                GitHub pull request. You get one clear answer wherever work is reviewed.
              </p>
            </div>
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
              <div className="font-serif text-lg text-text">A clear answer</div>
              <p className="mt-2 text-sm leading-relaxed text-text-dim">
                See exactly what needs attention, where it is, and why. Clean changes pass; risky
                changes are held for a human. Private repositories stay on the connection you provide.
              </p>
            </div>
          </div>
        </Reveal>
      </Section>

      {/* ---------- Benchmarks ---------- */}
      <Section
        id="benchmarks"
        title="Tested against the ways things go wrong"
        lede="Trace tests itself against dangerous commands, hidden secrets, and common workarounds. The numbers are repeatable, and the test is included with the CLI."
      >
        <Reveal>
          <div className="overflow-hidden rounded-2xl border border-border bg-[#0d0d10] p-6 font-mono text-[13px] leading-relaxed text-white">
            <div className="text-white/40">$ trc self-check</div>
            <div className="mt-3 text-white/70">Trace red-team detection benchmark</div>
            <div className="mt-1"><span className="text-emerald-400">59/59</span> dangerous examples stopped · <span className="text-emerald-400">0</span> safe examples interrupted</div>
            <div className="mt-3 space-y-1">
              <div>
                <span className="text-emerald-400">[PASS]</span> Dangerous commands&nbsp;&nbsp;35/35 stopped · 0 safe commands stopped
              </div>
              <div>
                <span className="text-emerald-400">[PASS]</span> Secrets&nbsp;&nbsp;18/18 found · 0 safe examples interrupted
              </div>
              <div>
                <span className="text-emerald-400">[PASS]</span> Unsafe instructions&nbsp;&nbsp;6/6 found
              </div>
            </div>
            <div className="mt-3 text-white/40">
              fixed examples included with the CLI
            </div>
            <div className="mt-3 text-emerald-400">
              All fixtures and red-team threats passed.
            </div>
          </div>
          <div className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3">
            <FeatureCard
              title="It catches workarounds too"
              body="Trace checks more than the obvious spelling, so a dangerous action cannot slip through just because it was written a different way."
            />
            <FeatureCard
              title="It does not cry wolf"
              body="Normal work, documentation, and harmless examples stay out of the way."
            />
            <FeatureCard
              title="Runs on every build"
              body="The examples are part of the test suite, so a safety regression is caught before release."
            />
          </div>
        </Reveal>
      </Section>

      {/* ---------- Dashboard ---------- */}
      <Section
        id="dashboard"
        title="Every session, laid out plainly"
        lede="Open the local dashboard and see the current run first: which agent is working, where it is working, what it costs, what passed, and what Trace stopped. Then follow the timeline, changes, safety decisions, spend, and undo points."
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
            <Button href="#install">Download the CLI</Button>
            <Button variant="secondary" href={DOCS_URL} target="_blank" rel="noreferrer">Read the documentation</Button>
          </div>
        </Reveal>
      </section>
    </>
  );
}

function FeatureCard({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-xl border border-border bg-white p-5 shadow-sm">
      <div className="font-serif text-lg text-text">{title}</div>
      <p className="mt-2 text-sm leading-relaxed text-text-dim">{body}</p>
    </div>
  );
}

/* A small, clickable preview of the local dashboard. It keeps the feeling of
   a live run without pretending these are the visitor's own records. */

const SIDEBAR = ["Overview", "Timeline", "Changes", "Safety", "Spend", "Undo changes"];

const SESSIONS = [
  { prompt: "Add pagination and tests", files: 5, risk: "low", cost: "$0.06", status: "completed", time: "4m ago" },
  { prompt: "Update the settings loader", files: 8, risk: "low", cost: "$0.09", status: "completed", time: "31m ago" },
  { prompt: "Download and run an unknown script", files: 0, risk: "high", cost: "-", status: "blocked", time: "1h ago" },
];

const TIMELINE = [
  { time: "10:42", event: "Saved an undo point" },
  { time: "10:42", event: "Started watching this run" },
  { time: "10:43", event: "Changed the user list" },
  { time: "10:44", event: "Added pagination" },
  { time: "10:45", event: "Tests passed" },
  { time: "10:45", event: "Run finished" },
];

const PATCH = [
  { path: "user list", add: 24, del: 6 },
  { path: "pagination", add: 41, del: 0 },
  { path: "user list tests", add: 18, del: 2 },
];

const COMMANDS = [
  { cmd: "Run the tests", risk: "low" },
  { cmd: "Save the changes", risk: "low" },
  { cmd: "Run an unknown download", risk: "high" },
];

const SPEND = [
  { model: "Main model", tokens: "12,400", cost: "$0.09" },
  { model: "Fast model", tokens: "3,100", cost: "$0.01" },
];

const CHECKPOINTS = [
  { ref: "Before this run", time: "4m ago" },
  { ref: "Before the last run", time: "31m ago" },
  { ref: "Before yesterday's run", time: "1h ago" },
];

function DashboardPreview() {
  const [page, setPage] = useState("Overview");

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-border px-5 py-3.5">
        <div className="flex gap-1.5">
          <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
          <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
          <span className="h-3 w-3 rounded-full bg-[#28c840]" />
        </div>
        <span className="ml-2 font-mono text-xs text-brand">Trace · live run</span>
      </div>

      <div className="grid grid-cols-[210px_1fr]">
        {/* sidebar: click an item to switch pages */}
        <div className="border-r border-border bg-surface p-4">
          <div className="mb-4 flex items-center gap-2 px-2 text-[15px] font-semibold text-text">
            <svg width="16" height="16" viewBox="0 0 194 200" fill="none">
              <rect x="31" y="32" width="132" height="30" rx="8" fill="#2f6fed" />
              <rect x="82" y="74" width="30" height="90" rx="8" fill="#2f6fed" />
            </svg>
            Trace
          </div>
          {SIDEBAR.map((name) => (
            <button
              key={name}
              onClick={() => setPage(name)}
              className={`mb-1 block w-full rounded-lg px-3 py-2 text-left text-[13.5px] transition-colors ${
                page === name
                  ? "bg-brand-soft font-medium text-brand-dim"
                  : "text-text-dim hover:bg-surface-2"
              }`}
            >
              {name}
            </button>
          ))}
        </div>

        {/* main content */}
        <div className="p-6">
          {page === "Overview" ? (
            <>
              {/* KPI row */}
              <div className="mb-6 grid grid-cols-4 gap-3">
                <KPI value="3" label="Sessions today" />
                <KPI value="13" label="Files changed" />
                <KPI value="1" label="Blocked" color="text-bad" />
                <KPI value="$0.15" label="Total cost" />
              </div>

              {/* recent sessions */}
              <div className="mb-3 text-[12px] font-medium uppercase tracking-wider text-text-dim">
                Recent runs
              </div>
              <div className="space-y-2.5">
                {SESSIONS.map((s, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between gap-4 rounded-lg border border-border px-4 py-3"
                  >
                    <div className="min-w-0">
                      <span className="truncate font-mono text-[13px] text-text">{s.prompt}</span>
                      <div className="mt-1 text-[11px] text-text-dim">{s.time} · {s.files} files</div>
                    </div>
                    <div className="flex shrink-0 items-center gap-3">
                      <RiskBadge level={s.risk} />
                      <span className="w-14 text-right font-mono text-[12px] text-text-dim">{s.cost}</span>
                      <StatusBadge status={s.status} />
                    </div>
                  </div>
                ))}
              </div>
            </>
          ) : page === "Timeline" ? (
            <PageBody title="What happened">
              {TIMELINE.map((t, i) => (
                <div key={i} className="flex gap-3 border-l-2 border-border pl-4">
                  <div className="w-16 shrink-0 font-mono text-[11px] text-text-dim">{t.time}</div>
                  <div className="pb-3 text-[13px] text-text">{t.event}</div>
                </div>
              ))}
            </PageBody>
          ) : page === "Changes" ? (
            <PageBody title="What changed">
              {PATCH.map((f) => (
                <div key={f.path} className="flex items-center justify-between rounded-lg border border-border px-4 py-2.5">
                  <span className="truncate font-mono text-[13px] text-text">{f.path}</span>
                  <span className="shrink-0 font-mono text-[12px]">
                    <span className="text-good">+{f.add}</span> <span className="text-bad">-{f.del}</span>
                  </span>
                </div>
              ))}
            </PageBody>
          ) : page === "Safety" ? (
            <PageBody title="What Trace allowed or stopped">
              {COMMANDS.map((c, i) => (
                <div key={i} className="flex items-center justify-between gap-3 rounded-lg border border-border px-4 py-2.5">
                  <span className="truncate font-mono text-[13px] text-text-dim">{c.cmd}</span>
                  <RiskBadge level={c.risk} />
                </div>
              ))}
            </PageBody>
          ) : page === "Spend" ? (
            <PageBody title="What the run cost">
              {SPEND.map((s, i) => (
                <div key={i} className="flex items-center justify-between rounded-lg border border-border px-4 py-2.5">
                  <div>
                    <div className="text-[13px] font-medium text-text">{s.model}</div>
                    <div className="text-[11px] text-text-dim">{s.tokens} tokens</div>
                  </div>
                  <span className="font-mono text-[13px] text-text">{s.cost}</span>
                </div>
              ))}
            </PageBody>
          ) : (
            <PageBody title="Undo points">
              {CHECKPOINTS.map((c, i) => (
                <div key={i} className="flex items-center justify-between gap-3 rounded-lg border border-border px-4 py-2.5">
                  <div>
                    <div className="font-mono text-[12px] text-text">{c.ref}</div>
                    <div className="text-[11px] text-text-dim">{c.time}</div>
                  </div>
                  <button className="btn-pop rounded-full border border-border px-3 py-1 text-[12px] font-medium text-text hover:border-brand hover:text-brand">
                    Roll back
                  </button>
                </div>
              ))}
            </PageBody>
          )}
        </div>
      </div>
    </div>
  );
}

function PageBody({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div>
      <div className="mb-3 text-[12px] font-medium uppercase tracking-wider text-text-dim">{title}</div>
      <div className="space-y-2.5">{children}</div>
    </div>
  );
}

function KPI({ value, label, color }: { value: string; label: string; color?: string }) {
  return (
    <div className="rounded-lg border border-border bg-surface px-4 py-3">
      <div className={`text-xl font-semibold ${color || "text-text"}`}>{value}</div>
      <div className="mt-0.5 text-[10.5px] uppercase tracking-wide text-text-dim">{label}</div>
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
