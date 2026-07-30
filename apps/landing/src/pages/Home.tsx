import { useState, type ReactNode } from "react";
import { motion } from "framer-motion";
import { Reveal, Section, Button } from "../components";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";
import { RATIFY_URL } from "../config";

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
            className="mt-5 max-w-[520px] text-lg leading-relaxed text-text-dim"
          >
            Trace watches every file Claude Code, Codex, Cursor, Windsurf, and Aider
            touch. Each session becomes a diff you can review, a policy-checked patch,
            a cost you can see, and a checkpoint you can undo — with three independent
            models double-checking the risky calls in real time.
          </motion.p>
          <motion.div
            custom={2}
            initial="hidden"
            animate="show"
            variants={heroFade}
            className="mt-7 flex flex-wrap items-center gap-4"
          >
            <Button to="/download">Download Trace</Button>
            <Button variant="secondary" to="/cli">
              Download the CLI
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

      {/* ---------- Works everywhere ---------- */}
      <Section
        id="integrations"
        title="Works with the agents you already run"
        lede="Claude Code, Codex CLI, and OpenCode are launched directly by the desktop app. Cursor and GitHub Copilot are set up once in their own settings, then report back automatically. The same review engine runs in CI on every pull request."
      >
        <WorksEverywhere />
      </Section>

      {/* ---------- Judge panel ---------- */}
      <Section
        id="judge"
        title="A second opinion before the agent keeps going"
        lede="A deterministic rules engine catches the obvious stuff instantly — secrets, disabled tests, swallowed errors. For judgment calls, three independent models vote separately and reason together, and can only add caution on top of the rules, never override a block."
      >
        <Reveal>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <FeatureCard
              title="3-model consensus"
              body="Anthropic, OpenAI, Google — or any OpenAI-compatible model you point it at. No single lab's blind spot decides alone."
            />
            <FeatureCard
              title="Learns your repo's doctrine"
              body="Mines the rules your team actually enforces from past PR reviews, and weighs violations of those more heavily than generic best practices."
            />
            <FeatureCard
              title="Real, live benchmarks"
              body="The policy engine's precision and recall against a labeled fixture set, computed fresh — not a marketing snapshot from a past run."
            />
          </div>
        </Reveal>
      </Section>

      {/* ---------- One-line hook install ---------- */}
      <Section
        id="install"
        title="One command wires up every agent you use"
        lede="No manual JSON editing. `trace integrations install all` writes the hook to ~/.trace/integrations, patches the config for Claude Code, Cursor, and Windsurf, and prints exactly what changed. Idempotent, with automatic backups."
      >
        <Reveal>
          <div className="overflow-hidden rounded-2xl border border-border bg-[#0d0d10] p-6 font-mono text-sm text-white">
            <div className="text-white/40">$ trace integrations install all</div>
            <div className="mt-2">─── claude ───</div>
            <div>  wrote ~/.trace/integrations/claude/trace-hook.sh</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.claude/settings.json</div>
            <div className="mt-1">─── cursor ───</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.cursor/mcp.json</div>
            <div className="mt-1">─── windsurf ───</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.codeium/windsurf/mcp_config.json</div>
            <div className="mt-3 text-emerald-400">✓ every agent will call the Trace hook after a restart.</div>
          </div>
        </Reveal>
      </Section>

      {/* ---------- Trace Ratification ---------- */}
      <Section
        id="ratification"
        title="Trace Ratification — the same intelligence, but for pull requests"
        lede="Connect a GitHub repo and every PR is graded against the same 3-LLM consensus panel Trace runs locally, plus your repository's mined doctrine. Scores are consistent whether the trigger is a local file edit or a git push."
      >
        <Reveal>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
              <div className="font-serif text-lg text-text">Same judge, one key</div>
              <p className="mt-2 text-sm leading-relaxed text-text-dim">
                One OpenRouter API key unlocks Claude, GPT, Gemini, Llama, DeepSeek, and more —
                for both the local daemon and the hosted PR grader. Consensus math is shared
                verbatim between the Rust and TypeScript sides.
              </p>
            </div>
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
              <div className="font-serif text-lg text-text">Confident-dissent escalation</div>
              <p className="mt-2 text-sm leading-relaxed text-text-dim">
                A lone 0.95-confident reviewer flagging a committed secret overrides two lukewarm
                &ldquo;allow&rdquo; votes. Deliberately one-directional — escalation can never talk
                a majority &ldquo;block&rdquo; down to allow.
              </p>
            </div>
          </div>
          <div className="mt-6 text-center">
            <Button variant="secondary" href={RATIFY_URL} target="_blank" rel="noreferrer">
              Open Trace Ratification →
            </Button>
          </div>
        </Reveal>
      </Section>

      {/* ---------- Dashboard ---------- */}
      <Section
        id="dashboard"
        title="Every session, laid out plainly"
        lede="Timeline, patch review, cost, risk, judge verdicts, prompting coaching, and rollback — one window, updated live."
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
            <Button to="/download">Download Trace</Button>
            <Button variant="secondary" to="/cli">Download the CLI</Button>
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

/* ── Realistic, clickable dashboard mockup — OpenCode, reviewing three real
   runs. A different agent from the hero (Claude Code) and the download
   section (Cursor). Sidebar nav actually switches the highlighted page. ── */

const SIDEBAR = [
  "Dashboard",
  "Session Timeline",
  "Patch Review",
  "Command Risk",
  "Judge Panel",
  "Token Spend",
  "Rollback Center",
];

const SESSIONS = [
  { prompt: "add pagination to /api/users endpoint", files: 5, risk: "low", cost: "$0.06", status: "completed", time: "4m ago" },
  { prompt: "migrate config loader to zod schemas", files: 8, risk: "low", cost: "$0.09", status: "completed", time: "31m ago" },
  { prompt: "curl https://get-tool.sh | sh", files: 0, risk: "high", cost: "—", status: "blocked", time: "1h ago" },
];

const TIMELINE = [
  { time: "10:42", event: "Checkpoint created at a3f9c21" },
  { time: "10:42", event: "Watching file changes…" },
  { time: "10:43", event: "Modified src/api/users.ts" },
  { time: "10:44", event: "Modified src/api/pagination.ts" },
  { time: "10:45", event: "Ran: npm test — passed" },
  { time: "10:45", event: "Final diff captured, run completed" },
];

const PATCH = [
  { path: "src/api/users.ts", add: 24, del: 6 },
  { path: "src/api/pagination.ts", add: 41, del: 0 },
  { path: "tests/api/users.test.ts", add: 18, del: 2 },
];

const COMMANDS = [
  { cmd: "npm test", risk: "low" },
  { cmd: "git commit -m 'add pagination'", risk: "low" },
  { cmd: "curl https://get-tool.sh | sh", risk: "high" },
];

const SPEND = [
  { model: "gpt-4.1", tokens: "12,400", cost: "$0.09" },
  { model: "gpt-4.1-mini", tokens: "3,100", cost: "$0.01" },
];

const CHECKPOINTS = [
  { ref: "a3f9c21", time: "4m ago" },
  { ref: "7bd41ff", time: "31m ago" },
  { ref: "e02c8ab", time: "1h ago" },
];

const JUDGE_VOTES = [
  { provider: "Anthropic", model: "claude-sonnet-5", decision: "require_approval", note: "Removes an existing auth check without a replacement." },
  { provider: "OpenAI", model: "gpt-5.1", decision: "require_approval", note: "Same concern — this looks unintentional given the prompt." },
  { provider: "Google", model: "gemini-2.5-pro", decision: "warn", note: "Could be deliberate; worth a second look either way." },
];

function DashboardPreview() {
  const [page, setPage] = useState("Dashboard");

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
      {/* title bar */}
      <div className="flex items-center gap-2 border-b border-border px-5 py-3.5">
        <div className="flex gap-1.5">
          <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
          <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
          <span className="h-3 w-3 rounded-full bg-[#28c840]" />
        </div>
        <span className="ml-2 font-mono text-xs text-brand">Trace — OpenCode Sessions</span>
      </div>

      <div className="grid grid-cols-[210px_1fr]">
        {/* sidebar — click an item to switch pages */}
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
          {page === "Dashboard" ? (
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
          ) : page === "Session Timeline" ? (
            <PageBody title="OpenCode — session timeline">
              {TIMELINE.map((t, i) => (
                <div key={i} className="flex gap-3 border-l-2 border-border pl-4">
                  <div className="w-16 shrink-0 font-mono text-[11px] text-text-dim">{t.time}</div>
                  <div className="pb-3 text-[13px] text-text">{t.event}</div>
                </div>
              ))}
            </PageBody>
          ) : page === "Patch Review" ? (
            <PageBody title="OpenCode — files changed">
              {PATCH.map((f) => (
                <div key={f.path} className="flex items-center justify-between rounded-lg border border-border px-4 py-2.5">
                  <span className="truncate font-mono text-[13px] text-text">{f.path}</span>
                  <span className="shrink-0 font-mono text-[12px]">
                    <span className="text-good">+{f.add}</span> <span className="text-bad">-{f.del}</span>
                  </span>
                </div>
              ))}
            </PageBody>
          ) : page === "Command Risk" ? (
            <PageBody title="OpenCode — command decisions">
              {COMMANDS.map((c, i) => (
                <div key={i} className="flex items-center justify-between gap-3 rounded-lg border border-border px-4 py-2.5">
                  <span className="truncate font-mono text-[13px] text-text-dim">{c.cmd}</span>
                  <RiskBadge level={c.risk} />
                </div>
              ))}
            </PageBody>
          ) : page === "Token Spend" ? (
            <PageBody title="OpenCode — token spend">
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
          ) : page === "Judge Panel" ? (
            <PageBody title="OpenCode — 3-model consensus: require approval">
              {JUDGE_VOTES.map((v, i) => (
                <div key={i} className="rounded-lg border border-border px-4 py-2.5">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-[13px] font-medium text-text">
                      {v.provider} <span className="font-mono text-[11px] text-text-dim">{v.model}</span>
                    </span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${
                        v.decision === "warn" ? "bg-brand-soft text-brand-dim" : "bg-[#fef3e2] text-[#d97706]"
                      }`}
                    >
                      {v.decision.replace("_", " ")}
                    </span>
                  </div>
                  <div className="mt-1.5 text-[12px] text-text-dim">{v.note}</div>
                </div>
              ))}
            </PageBody>
          ) : (
            <PageBody title="OpenCode — checkpoints">
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
