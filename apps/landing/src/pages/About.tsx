import { Link } from "react-router-dom";

export default function About() {
  return (
    <div className="max-w-[680px] py-16">
      <Link to="/" className="text-sm text-text-dim hover:text-text">← Back</Link>
      <h1 className="mt-4 text-3xl font-semibold">About Trace</h1>
      <p className="mt-3 text-text-dim">
        Git stores commits. Trace stores execution — built for developers who
        refuse to ship code they never saw.
      </p>

      <Prose title="What Trace is">
        Trace is a black box recorder and AI safety layer for coding agents.
        You launch an agent or command through the <code>trace</code> wrapper,
        and Trace turns the session into a reviewable record: what changed, what
        ran, what looked risky, what it cost, what broke, and how to roll back —
        all from a dashboard that runs on your own machine.
      </Prose>

      <Prose title="Why it exists">
        AI agents edit files, run shell commands, touch secrets, change
        dependencies, and spend tokens faster than any human can follow. Left
        unwatched, an agent run is a black box. Trace replaces that black
        box with an execution timeline, patch intelligence, command risk
        analysis, and rollback-ready checkpoints — so you stay in control of
        automation instead of cleaning up after it.
      </Prose>

      <Prose title="What it records">
        <ul className="space-y-1.5">
          <li>A full session timeline: prompts, commands, file edits, checks, results</li>
          <li>Patch review from real Git diffs — added, modified, deleted, config, env</li>
          <li>Command risk: destructive, unsafe, or suspicious shell behavior</li>
          <li>Secret detection with redaction — raw values are never stored</li>
          <li>Token spend: provider, model, usage, and estimated cost when available</li>
          <li>Git-backed rollback points created before monitored runs</li>
        </ul>
      </Prose>

      <Prose title="Local-first by design">
        Trace's real dashboard runs only on <code>127.0.0.1</code>. Project
        history, Git diffs, and run records stay on your machine by default.
        There is no account and no cloud sync. Raw secrets are never stored,
        and any cloud or GitHub integration uses sanitized summaries only.
      </Prose>

      <Prose title="What it does not do">
        <ul className="space-y-1.5">
          <li>It does not replace your AI coding agent or write code for you.</li>
          <li>It does not upload your project, secrets, or local database.</li>
          <li>It does not require a login or a cloud account.</li>
          <li>
            It does not pretend GUI tools are fully controllable — full guarding
            requires the <code>trace</code> wrapper or supported hooks.
          </li>
        </ul>
      </Prose>

      <Prose title="Integration status">
        Honest by default. Terminal agents (Claude Code, Codex, and generic
        commands) work today through <code>trace run</code>. The Cursor MCP
        server and VS Code extension are built and load locally. GitHub
        Actions CI summaries are available; a GitHub App with PR checks is on
        the roadmap.
      </Prose>

      <Prose title="Roadmap: teams and desktop">
        Trace is developer-controlled and local-first today. A native desktop
        app is next. A future team tier — shared policy rules, organization
        dashboards, sanitized run summaries, AI activity audit logs,
        role-based access, and centralized reporting — is on the roadmap and
        is not available now.
      </Prose>
    </div>
  );
}

function Prose({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mt-10 border-t border-border pt-6">
      <h2 className="text-xl font-semibold">{title}</h2>
      <div className="mt-2.5 text-text-dim [&_code]:text-text [&_li]:list-disc [&_li]:ml-5">
        {children}
      </div>
    </section>
  );
}
