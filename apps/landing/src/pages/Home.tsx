import { useState, type ReactNode } from "react";
import { motion } from "framer-motion";
import { Reveal, Section, Button } from "../components";
import HeroDemo from "../HeroDemo";
import WorksEverywhere from "../WorksEverywhere";

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
            Trace watches every file Claude Code, Codex, Cursor, Windsurf, opencode,
            and Aider touch. Each session becomes a diff you can review, a
            policy-checked patch, a cost you can see, and a checkpoint you can undo —
            guarded in real time by a deterministic engine that runs entirely on your
            machine, with no API key.
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
        lede="Claude Code, Codex CLI, and opencode connect through Trace's hooks and local MCP server. Cursor, Windsurf, and VS Code are set up once in their own settings, then report back automatically. The same review engine runs in CI on every pull request."
      >
        <WorksEverywhere />
      </Section>

      {/* ---------- One-line hook install ---------- */}
      <Section
        id="install"
        title="One command wires up every agent you use"
        lede="No manual JSON editing. `trc integrations install all` writes the hook to ~/.trace/integrations, patches the config for Claude Code, Codex, Cursor, Windsurf, and opencode, and prints exactly what changed. Idempotent, with automatic backups."
      >
        <Reveal>
          <div className="overflow-hidden rounded-2xl border border-border bg-[#0d0d10] p-6 font-mono text-sm text-white">
            <div className="text-white/40">$ trc integrations install all</div>
            <div className="mt-2">─── claude ───</div>
            <div>  wrote ~/.trace/integrations/claude/trace-hook.sh</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.claude/settings.json</div>
            <div className="mt-1">─── codex ───</div>
            <div>  wrote ~/.trace/integrations/codex/codex-adapter.sh</div>
            <div className="mt-1">─── cursor ───</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.cursor/mcp.json</div>
            <div className="mt-1">─── windsurf ───</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.codeium/windsurf/mcp_config.json</div>
            <div className="mt-1">─── opencode ───</div>
            <div>  <span className="text-emerald-400">patched</span> ~/.config/opencode/opencode.json</div>
            <div className="mt-3 text-emerald-400">✓ every agent will call the Trace hook after a restart.</div>
          </div>
        </Reveal>
      </Section>

      {/* ---------- Trace Ratification ---------- */}
      <Section
        id="ratification"
        title="Ratify a pull request — right from the local dashboard"
        lede="Connect a GitHub repo and ratify any PR against the exact same policy engine that guards your local edits: secret scanning, risky-change detection, disabled-test checks, and more. Pure pattern matching — no LLM, no API key — so every verdict is instant, free, and identical for everyone."
      >
        <Reveal>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
              <div className="font-serif text-lg text-text">One engine, edit to PR</div>
              <p className="mt-2 text-sm leading-relaxed text-text-dim">
                The same deterministic rules run on a local file edit, in CI via{" "}
                <span className="font-mono text-[13px]">trc review-diff</span>, and on a
                GitHub pull request from the dashboard's Ratify tab. Consistent by construction —
                one implementation, no drift.
              </p>
            </div>
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
              <div className="font-serif text-lg text-text">A clear, honest verdict</div>
              <p className="mt-2 text-sm leading-relaxed text-text-dim">
                A PR is <b>block</b> if it trips any high-severity rule, <b>needs review</b> for
                medium-only, else <b>pass</b> — with every finding, its file, and its severity
                listed. Reads private repos with a token that only ever touches api.github.com.
              </p>
            </div>
          </div>
        </Reveal>
      </Section>

      {/* ---------- Benchmarks ---------- */}
      <Section
        id="benchmarks"
        title="Measured against an adversarial corpus, not vibes"
        lede="Trace ships a labeled red-team corpus — dangerous commands (including evasions like curl … | sudo bash and base64-piped shells), planted API keys, and unsafe prompts — run through the exact guard, secret, and prompt engines the runtime hook uses. Reproduce every number yourself with `trc self-check`."
      >
        <Reveal>
          <div className="overflow-hidden rounded-2xl border border-border bg-[#0d0d10] p-6 font-mono text-[13px] leading-relaxed text-white">
            <div className="text-white/40">$ trc self-check</div>
            <div className="mt-3 text-white/70">Trace red-team detection benchmark</div>
            <div className="mt-1">
              <span className="text-emerald-400">59/59</span> threats caught
              &nbsp;·&nbsp; <span className="text-emerald-400">0</span> false
              positives &nbsp;·&nbsp; recall{" "}
              <span className="text-emerald-400">100%</span>
            </div>
            <div className="mt-3 space-y-1">
              <div>
                <span className="text-emerald-400">[PASS]</span> Command guard
                &nbsp;&nbsp;&nbsp;&nbsp;35/35 caught · 0 missed · 0 false+ (7 benign)
              </div>
              <div>
                <span className="text-emerald-400">[PASS]</span> Secret detection
                &nbsp;18/18 caught · 0 missed · 0 false+ (3 benign)
              </div>
              <div>
                <span className="text-emerald-400">[PASS]</span> Prompt risk
                &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;6/6 caught · 0 missed · 0 false+ (2 benign)
              </div>
            </div>
            <div className="mt-3 text-white/40">
              rule pack 2025.08.1 · 29 injection phrases · 3 command rules · 2
              secret patterns
            </div>
            <div className="mt-3 text-emerald-400">
              All fixtures and red-team threats passed.
            </div>
          </div>
          <div className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3">
            <FeatureCard
              title="Evasions, not just the obvious"
              body="Pipe-to-sudo-shell, download-then-exec, base64-decoded payloads, find -delete, and raw block-device writes are all caught — the tricks that slip past a naive substring blocklist."
            />
            <FeatureCard
              title="Zero false positives"
              body="Benign look-alikes — a commit message mentioning “drop table”, a docs URL, clean source — stay clean. Recall means nothing if the tool cries wolf on real work."
            />
            <FeatureCard
              title="Runs on every build"
              body="The corpus is a unit test and a CI gate. A regression that lets a threat through — or trips on something safe — fails the build before it ships."
            />
          </div>
        </Reveal>
      </Section>

      {/* ---------- Dashboard ---------- */}
      <Section
        id="dashboard"
        title="Every session, laid out plainly"
        lede="Timeline, patch review, cost, command risk, PR ratification, benchmarks, and rollback — one window, updated live."
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
  "Ratify",
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

const RATIFY_FINDINGS = [
  { severity: "high", title: "Hardcoded secret in diff", file: "src/config/prod.ts", note: "An AWS access key id was added on line 12." },
  { severity: "medium", title: "Test file removed", file: "tests/api/users.test.ts", note: "Coverage for a changed endpoint was deleted." },
  { severity: "low", title: "Debug TODO left in", file: "src/api/pagination.ts", note: "A `// TODO: remove` marker shipped in the change." },
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
          ) : page === "Ratify" ? (
            <PageBody title="acme-webapp — ratify PR #142: block">
              {RATIFY_FINDINGS.map((f, i) => (
                <div key={i} className="rounded-lg border border-border px-4 py-2.5">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-[13px] font-medium text-text">
                      {f.title} <span className="font-mono text-[11px] text-text-dim">{f.file}</span>
                    </span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${
                        f.severity === "high"
                          ? "bg-[#fdeaea] text-[#dc2626]"
                          : f.severity === "medium"
                          ? "bg-[#fef3e2] text-[#d97706]"
                          : "bg-brand-soft text-brand-dim"
                      }`}
                    >
                      {f.severity}
                    </span>
                  </div>
                  <div className="mt-1.5 text-[12px] text-text-dim">{f.note}</div>
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
