import { Link } from "react-router-dom";
import { Cmd } from "../components";
import Download from "../Download";
import { DOCS_URL, GITHUB_REPO } from "../config";

/** Minimal inline icons (stroke-based) keyed by concept. */
function Icon({ name }: { name: string }) {
  const p: Record<string, string> = {
    timeline: "M4 6h16M4 12h10M4 18h7",
    patch: "M4 4h10l6 6v10H4z M14 4v6h6",
    guard: "M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z",
    secret: "M7 11V8a5 5 0 0110 0v3 M5 11h14v9H5z",
    cost: "M12 2v20 M17 6H9.5a3.5 3.5 0 100 7h5a3.5 3.5 0 110 7H6",
    rollback: "M3 12a9 9 0 109-9 M3 12V6 M3 12h6",
  };
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d={p[name] ?? p.timeline} />
    </svg>
  );
}

const FEATURES: { icon: string; title: string; desc: string; bullets: string[] }[] = [
  {
    icon: "timeline",
    title: "Agent Run Recorder",
    desc: "Every AI coding session becomes a readable execution timeline.",
    bullets: ["Prompts, commands, edits, checks, results", "Exact sequence, not guesswork", "One run, one reviewable record"],
  },
  {
    icon: "patch",
    title: "Patch Review",
    desc: "The actual code delta from an AI run, in one view.",
    bullets: ["Added, modified, deleted files", "Dependency, config & env changes", "Real Git diffs — nothing invented"],
  },
  {
    icon: "guard",
    title: "Command Guard",
    desc: "Rule-based defense against dangerous shell behavior.",
    bullets: ["Destructive deletes, hard resets", "Unsafe perms, piped install scripts", "Warn, block, or require approval"],
  },
  {
    icon: "secret",
    title: "Secret Detection",
    desc: "Catches exposed credentials before they're stored.",
    bullets: ["API keys, tokens, SSH keys, DB URLs", "Scans diffs, output & changed files", "Redacted before storage — always"],
  },
  {
    icon: "cost",
    title: "Cost Center",
    desc: "Makes AI usage visible when adapter data is available.",
    bullets: ["Provider, model, tokens, latency", "Estimated cost per run", "Honest 'unavailable' when unknown"],
  },
  {
    icon: "rollback",
    title: "Rollback Center",
    desc: "Git-backed checkpoints before every monitored run.",
    bullets: ["Review, reject, or roll back", "Confirmed — never destructive silently", "Requires Git in the project"],
  },
];

const INTEGRATIONS: { name: string; note: string; status: "live" | "ready" | "planned" }[] = [
  { name: "Claude Code", note: "wrapper + hooks adapter", status: "live" },
  { name: "Codex CLI", note: "wrapper", status: "live" },
  { name: "Generic terminal agents", note: "trace run <command>", status: "live" },
  { name: "Cursor", note: "MCP server (load locally)", status: "ready" },
  { name: "VS Code", note: "extension (load unpacked)", status: "ready" },
  { name: "GitHub Actions", note: "CI summary workflow", status: "ready" },
  { name: "GitHub App / PR checks", note: "skeleton", status: "planned" },
];

const ROADMAP = [
  "Native desktop app (macOS first)",
  "Shared policy rules",
  "Organization dashboards",
  "GitHub organization integration",
  "Sanitized run summaries",
  "AI activity audit logs",
  "Role-based access",
  "Centralized reporting",
];

export default function Home() {
  return (
    <>
      {/* ---------- Hero ---------- */}
      <section className="hero-grid">
        <div>
          <span className="hero-eyebrow">
            <span className="pulse" /> Local-first · v1.1
          </span>
          <h1 className="hero-title">
            The trust layer for <span className="grad">AI software engineering.</span>
          </h1>
          <p className="hero-sub">
            Trace records every AI coding session — <b>file changes, shell
            commands, risky actions, token cost, build results, and rollback
            checkpoints</b> — from a local-first dashboard you control.
          </p>
          <div className="hero-actions">
            <a className="btn-primary" href={DOCS_URL} target="_blank" rel="noreferrer">
              Read the docs
            </a>
            <a className="btn-ghost" href="#download">
              Install the CLI
            </a>
          </div>
          <div className="hero-install">
            <div className="label">Get started</div>
            <Cmd>brew install trace</Cmd>
          </div>
        </div>

        <DashboardMockup />
      </section>

      {/* ---------- Proof cards ---------- */}
      <section className="proof">
        <div className="proof-card">
          <div className="pc-t">Record every run</div>
          <div className="pc-d">A black box recorder for AI-generated code — commands, edits, costs, and checks in one timeline.</div>
        </div>
        <div className="proof-card">
          <div className="pc-t">Review every patch</div>
          <div className="pc-d">See the exact code delta before it ships. Real Git diffs, dependency and config changes surfaced.</div>
        </div>
        <div className="proof-card">
          <div className="pc-t">Roll back safely</div>
          <div className="pc-d">Git-backed checkpoints before each monitored run. Reject risky AI edits with one command.</div>
        </div>
      </section>

      {/* ---------- Feature matrix ---------- */}
      <section className="section" id="features">
        <div className="kicker">What Trace records</div>
        <h2>An execution trace for the AI-agent era</h2>
        <p className="lede">
          Stop treating agent runs like black boxes. Trace captures the full
          execution timeline, the patch, the risks, and the cost — locally.
        </p>
        <div className="matrix">
          {FEATURES.map((f) => (
            <div className="fcard" key={f.title}>
              <div className="ic"><Icon name={f.icon} /></div>
              <h3>{f.title}</h3>
              <p className="fd">{f.desc}</p>
              <ul>{f.bullets.map((b) => <li key={b}>{b}</li>)}</ul>
            </div>
          ))}
        </div>
      </section>

      {/* ---------- Integrations ---------- */}
      <section className="section">
        <div className="kicker">Integrations</div>
        <h2>Agent-aware, where you already work</h2>
        <p className="lede">
          Terminal agents run through <code>trace run</code> today. Editor
          adapters are built and load locally. Status is labeled honestly.
        </p>
        <div className="intg">
          {INTEGRATIONS.map((i) => (
            <div className="intg-card" key={i.name}>
              <div className="n">{i.name}<small>{i.note}</small></div>
              <span className={`status ${i.status}`}>
                {i.status === "live" ? "Available" : i.status === "ready" ? "Adapter-ready" : "Planned"}
              </span>
            </div>
          ))}
        </div>
      </section>

      {/* ---------- Local-first security ---------- */}
      <section className="section">
        <div className="kicker">Local-first security</div>
        <h2>Zero-cloud by default</h2>
        <div className="sec-grid">
          <div className="callout">
            <h3>Your machine. Your data.</h3>
            <p style={{ margin: 0 }}>
              Trace's real dashboard runs locally at <code>127.0.0.1</code>.
              Project history, Git diffs, and run records stay on your machine
              by default. Raw secrets are never stored — detections are
              redacted. Any cloud or GitHub integration uses sanitized
              summaries only.
            </p>
          </div>
          <ul className="sec-points">
            <li><b>Local daemon</b> bound to 127.0.0.1 — never the network</li>
            <li><b>No account</b>, no cloud sync, no telemetry by default</li>
            <li><b>Redacted secrets</b> — raw values never persisted</li>
            <li><b>Sanitized summaries</b> for CI — never raw files or DB</li>
          </ul>
        </div>
      </section>

      {/* ---------- Download ---------- */}
      <Download />

      {/* ---------- Future teams ---------- */}
      <section className="section">
        <div className="kicker">Roadmap</div>
        <h2>Built for solo developers first. Teams next.</h2>
        <p className="lede">
          Trace is developer-controlled and local-first today. A future team
          tier is on the roadmap — not available now.
        </p>
        <div className="roadmap">
          <span className="rm-tag">Future roadmap · not available yet</span>
          <ul>{ROADMAP.map((r) => <li key={r}>{r}</li>)}</ul>
        </div>
      </section>

      {/* ---------- Closing band ---------- */}
      <section className="band">
        <h2>See every AI edit before it ships.</h2>
        <p>Review the diff. Check the cost. Roll back safely.</p>
        <div className="hero-actions" style={{ justifyContent: "center" }}>
          <a className="btn-primary" href="#download">Install the CLI</a>
          <a className="btn-ghost" href={GITHUB_REPO} target="_blank" rel="noreferrer">View on GitHub</a>
          <Link className="btn-ghost" to="/about">About</Link>
        </div>
      </section>
    </>
  );
}

/** Static dashboard mockup for the hero (illustrative, not live data). */
function DashboardMockup() {
  return (
    <div className="mockup" aria-hidden="true">
      <div className="mockup-bar">
        <span className="mockup-dot" style={{ background: "#f85149" }} />
        <span className="mockup-dot" style={{ background: "#d29922" }} />
        <span className="mockup-dot" style={{ background: "#3fb950" }} />
        <span className="mb-title">127.0.0.1:8757 — Trace</span>
      </div>
      <div className="mockup-body">
        <div className="mk-side">
          <div className="mk-nav on">Dashboard</div>
          <div className="mk-nav">Session Timeline</div>
          <div className="mk-nav">Patch Intelligence</div>
          <div className="mk-nav">Command Risk</div>
          <div className="mk-nav">Token Spend</div>
          <div className="mk-nav">Rollback Points</div>
        </div>
        <div className="mk-main">
          <div className="mk-row">
            <span className="mk-cmd">claude "fix the login bug"</span>
            <span className="mk-badge ok">completed</span>
          </div>
          <div className="mk-row">
            <span className="mk-cmd">npm test</span>
            <span className="mk-badge ok">passed</span>
          </div>
          <div className="mk-row">
            <span className="mk-cmd">rm -rf dist</span>
            <span className="mk-badge warn">approval</span>
          </div>
          <div className="mk-row">
            <span className="mk-cmd">curl https://x.sh | sh</span>
            <span className="mk-badge block">blocked</span>
          </div>
          <div className="mk-stats">
            <div className="mk-stat"><div className="v">7</div><div className="k">files</div></div>
            <div className="mk-stat"><div className="v">1</div><div className="k">secret</div></div>
            <div className="mk-stat"><div className="v">$0.04</div><div className="k">est. cost</div></div>
          </div>
        </div>
      </div>
    </div>
  );
}
