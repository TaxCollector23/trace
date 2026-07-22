import { Link } from "react-router-dom";
import {
  ArrowCounterClockwise,
  ChartLine,
  GitDiff,
  ListBullets,
  LockKey,
  ShieldCheck,
} from "@phosphor-icons/react";
import { Cmd, Reveal, Section } from "../components";
import Download from "../Download";
import HeroDemo from "../HeroDemo";
import { DOCS_URL, GITHUB_REPO } from "../config";

const FEATURES = [
  {
    icon: ListBullets,
    title: "Agent run recorder",
    desc: "Every AI coding session becomes a readable execution timeline — prompts, commands, edits, checks, and results, in the exact order they happened.",
  },
  {
    icon: GitDiff,
    title: "Patch review",
    desc: "The real Git diff for an AI run in one place: added, modified, and deleted files, plus dependency and config changes. Nothing invented.",
  },
  {
    icon: ShieldCheck,
    title: "Command guard",
    desc: "Rule-based checks against destructive deletes, hard resets, unsafe permissions, and piped install scripts. Warn, block, or require approval.",
  },
  {
    icon: LockKey,
    title: "Secret detection",
    desc: "Scans diffs, output, and changed files for exposed API keys, tokens, and credentials. Redacted before anything is stored.",
  },
  {
    icon: ChartLine,
    title: "Cost tracking",
    desc: "Provider, model, tokens, and estimated cost per run when adapter data is available — an honest \"unavailable\" when it isn't.",
  },
  {
    icon: ArrowCounterClockwise,
    title: "Rollback",
    desc: "Git-backed checkpoints created before every monitored run. Review, reject, or roll back — never destructive without confirmation.",
  },
];

const INTEGRATIONS: { name: string; note: string; status: "Available" | "Adapter-ready" | "Planned" }[] = [
  { name: "Claude Code", note: "wrapper + hooks adapter", status: "Available" },
  { name: "Codex CLI", note: "wrapper", status: "Available" },
  { name: "Generic terminal agents", note: "trace run <command>", status: "Available" },
  { name: "Cursor", note: "MCP server", status: "Adapter-ready" },
  { name: "VS Code", note: "extension", status: "Adapter-ready" },
  { name: "GitHub Actions", note: "CI summary workflow", status: "Adapter-ready" },
  { name: "GitHub App / PR checks", note: "skeleton", status: "Planned" },
];

const ROADMAP = [
  "Native desktop app (macOS first)",
  "Transparent local cost proxy",
  "Shared policy rules",
  "Organization dashboards",
  "GitHub organization integration",
  "AI activity audit logs",
  "Role-based access",
  "Centralized reporting",
];

const statusColor: Record<string, string> = {
  Available: "text-good",
  "Adapter-ready": "text-brand",
  Planned: "text-text-dim",
};

export default function Home() {
  return (
    <>
      {/* ---------- Hero ---------- */}
      <section className="grid grid-cols-1 items-center gap-10 py-16 md:grid-cols-[1.05fr_1fr] md:py-24">
        <div>
          <h1 className="text-3xl font-semibold md:text-4xl">
            The trust layer for AI software engineering.
          </h1>
          <p className="mt-4 max-w-[520px] text-lg text-text-dim">
            Trace records every AI coding session — file changes, shell
            commands, risky actions, token cost, build results, and rollback
            checkpoints — from a local-first dashboard you control.
          </p>
          <div className="mt-6 flex flex-wrap gap-3">
            <a
              href={DOCS_URL}
              target="_blank"
              rel="noreferrer"
              className="rounded bg-brand px-5 py-2.5 text-sm font-medium text-white hover:bg-brand-dim"
            >
              Read the docs
            </a>
            <a
              href="#download"
              className="rounded border border-border px-5 py-2.5 text-sm font-medium text-text hover:border-brand-dim"
            >
              Install the CLI
            </a>
          </div>
          <div className="mt-8 max-w-[440px]">
            <div className="mb-1.5 text-xs uppercase tracking-wide text-text-dim">Get started</div>
            <Cmd>npm install -g trace</Cmd>
          </div>
        </div>

        <Reveal>
          <HeroDemo />
        </Reveal>
      </section>

      {/* ---------- Proof line ---------- */}
      <div className="grid grid-cols-1 gap-6 border-t border-border py-8 sm:grid-cols-3">
        <ProofItem title="Record every run" desc="A black box recorder for AI-generated code — commands, edits, costs, and checks in one timeline." />
        <ProofItem title="Review every patch" desc="See the exact code delta before it ships, grounded in the real Git diff." />
        <ProofItem title="Roll back safely" desc="Git-backed checkpoints before each monitored run. Reject risky AI edits with one command." />
      </div>

      {/* ---------- Download ---------- */}
      <Download />

      {/* ---------- Dashboard preview ---------- */}
      <Section
        title="Everything lands in one dashboard"
        lede="The same run, reviewable afterward — timeline, patch, risk, and cost in one place, served locally at 127.0.0.1."
      >
        <Reveal>
          <DashboardMockup />
        </Reveal>
      </Section>

      {/* ---------- Feature grid ---------- */}
      <Section
        id="features"
        title="An execution trace for the AI-agent era"
        lede="Stop treating agent runs like black boxes. Trace captures the full execution timeline, the patch, the risks, and the cost — locally."
      >
        <div className="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((f, i) => (
            <Reveal key={f.title} delay={i * 0.04}>
              <f.icon size={20} className="text-brand" />
              <h3 className="mt-3 text-base font-semibold">{f.title}</h3>
              <p className="mt-1.5 text-sm text-text-dim">{f.desc}</p>
            </Reveal>
          ))}
        </div>
      </Section>

      {/* ---------- Integrations ---------- */}
      <Section
        title="Agent-aware, where you already work"
        lede="Terminal agents run through trace run today. Editor adapters are built and load locally. Status is labeled honestly."
      >
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-border text-left text-xs uppercase tracking-wide text-text-dim">
              <th className="py-2 font-medium">Integration</th>
              <th className="py-2 font-medium">Type</th>
              <th className="py-2 font-medium">Status</th>
            </tr>
          </thead>
          <tbody>
            {INTEGRATIONS.map((i) => (
              <tr key={i.name} className="hover:bg-surface/60">
                <td className="py-3 font-medium">{i.name}</td>
                <td className="py-3 text-text-dim">{i.note}</td>
                <td className={`py-3 ${statusColor[i.status]}`}>{i.status}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Section>

      {/* ---------- Local-first security ---------- */}
      <Section title="Zero-cloud by default">
        <div className="grid grid-cols-1 gap-8 md:grid-cols-[1.3fr_1fr]">
          <p className="text-text-dim">
            Trace's real dashboard runs locally at{" "}
            <code className="text-text">127.0.0.1</code>. Project history,
            Git diffs, and run records stay on your machine by default. Raw
            secrets are never stored — detections are redacted. Any cloud or
            GitHub integration uses sanitized summaries only.
          </p>
          <ul className="space-y-3 text-sm text-text-dim">
            <li><span className="text-text">Local daemon</span> bound to 127.0.0.1 — never the network</li>
            <li><span className="text-text">No account</span>, no cloud sync, no telemetry by default</li>
            <li><span className="text-text">Redacted secrets</span> — raw values never persisted</li>
            <li><span className="text-text">Sanitized summaries</span> for CI — never raw files or the database</li>
          </ul>
        </div>
      </Section>

      {/* ---------- Roadmap ---------- */}
      <Section
        title="Built for solo developers first. Teams next."
        lede="Trace is developer-controlled and local-first today. A future team tier is on the roadmap — not available now."
      >
        <ul className="grid grid-cols-1 gap-x-8 gap-y-3 text-sm text-text-dim sm:grid-cols-2">
          {ROADMAP.map((r) => (
            <li key={r}>{r}</li>
          ))}
        </ul>
      </Section>

      {/* ---------- Closing ---------- */}
      <section className="border-t border-border py-16 text-center">
        <h2 className="text-2xl font-semibold">See every AI edit before it ships.</h2>
        <p className="mt-2 text-text-dim">Review the diff. Check the cost. Roll back safely.</p>
        <div className="mt-6 flex flex-wrap justify-center gap-3">
          <a href="#download" className="rounded bg-brand px-5 py-2.5 text-sm font-medium text-white hover:bg-brand-dim">
            Install the CLI
          </a>
          <a href={GITHUB_REPO} target="_blank" rel="noreferrer" className="rounded border border-border px-5 py-2.5 text-sm font-medium text-text hover:border-brand-dim">
            View on GitHub
          </a>
          <Link to="/about" className="rounded border border-border px-5 py-2.5 text-sm font-medium text-text hover:border-brand-dim">
            About
          </Link>
        </div>
      </section>
    </>
  );
}

function ProofItem({ title, desc }: { title: string; desc: string }) {
  return (
    <div>
      <div className="text-base font-semibold">{title}</div>
      <div className="mt-1 text-sm text-text-dim">{desc}</div>
    </div>
  );
}

const NAV_ITEMS = ["Dashboard", "Session Timeline", "Patch Review", "Command Risk", "Token Spend", "Rollback"];

/** Static dashboard mockup — mirrors the real dashboard's own layout and
 * type scale (see apps/web), not a stylized "hero graphic". No fake browser
 * chrome; the daemon URL is the only "window" cue, same as the real app. */
function DashboardMockup() {
  return (
    <div className="overflow-hidden rounded-md border border-border bg-surface" aria-hidden="true">
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
