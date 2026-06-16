import { Link } from "react-router-dom";

export default function About() {
  return (
    <div className="prose">
      <div className="page-head">
        <Link to="/" className="back">
          ← Back
        </Link>
        <h1>About TraceGuard</h1>
        <p className="muted">
          The local-first command center for the AI-agent era — built for
          developers who refuse to ship code they never saw.
        </p>
      </div>

      <h2>What TraceGuard is</h2>
      <p>
        TraceGuard is a black box recorder and AI safety layer for coding agents.
        You launch an agent or command through the <code>trg</code> wrapper, and
        TraceGuard turns the session into a reviewable record: what changed, what
        ran, what looked risky, what it cost, what broke, and how to roll back —
        all from a dashboard that runs on your own machine.
      </p>

      <h2>Why it exists</h2>
      <p>
        AI agents edit files, run shell commands, touch secrets, change
        dependencies, and spend tokens faster than any human can follow. Left
        unwatched, an agent run is a black box. TraceGuard replaces that black
        box with an execution timeline, patch intelligence, command risk
        analysis, and rollback-ready checkpoints — so you stay in control of
        automation instead of cleaning up after it.
      </p>

      <h2>What it records</h2>
      <ul>
        <li>A full session timeline: prompts, commands, file edits, checks, results</li>
        <li>Patch review from real Git diffs — added, modified, deleted, config, env</li>
        <li>Command risk: destructive, unsafe, or suspicious shell behavior</li>
        <li>Secret detection with redaction — raw values are never stored</li>
        <li>Token spend: provider, model, usage, and estimated cost when available</li>
        <li>Git-backed rollback points created before monitored runs</li>
        <li>TraceCompress: prompt compression and Bare Mode output discipline</li>
      </ul>

      <h2>TraceCompress</h2>
      <p>
        Developers waste tokens on bloated prompts; models waste tokens on filler,
        sugarcoating, and false completion claims. TraceCompress rewrites prompts
        into precise instructions while preserving meaning, file names, commands,
        and must-not-do rules — then attaches Bare Mode rules that force direct,
        minimal, verifiable output. It runs locally and deterministically by
        default; external-LLM compression is opt-in.
      </p>

      <h2>Local-first by design</h2>
      <p>
        TraceGuard's real dashboard runs only on <code>127.0.0.1</code>. Project
        history, Git diffs, prompt-compression metadata, and run records stay on
        your machine by default. There is no account and no cloud sync. Raw
        secrets are never stored, and any cloud or GitHub integration uses
        sanitized summaries only.
      </p>

      <h2>What it does not do</h2>
      <ul>
        <li>It does not replace your AI coding agent or write code for you.</li>
        <li>It does not upload your project, secrets, or local database.</li>
        <li>It does not require a login or a cloud account.</li>
        <li>
          It does not pretend GUI tools are fully controllable — full guarding
          requires the <code>trg</code> wrapper or supported hooks.
        </li>
      </ul>

      <h2>Integration status</h2>
      <p>
        Honest by default. Terminal agents (Claude Code, Codex, and generic
        commands) work today through <code>trg run</code>. The Cursor MCP server,
        VS Code extension, and browser extension for ChatGPT/Claude/Gemini are
        built and load locally. GitHub Actions CI summaries are available; a
        GitHub App with PR checks is on the roadmap.
      </p>

      <h2>Roadmap: teams</h2>
      <p>
        TraceGuard is developer-controlled and local-first today. A future team
        tier — shared policy rules, organization dashboards, sanitized run
        summaries, AI activity audit logs, role-based access, and centralized
        reporting — is on the roadmap and is not available now.
      </p>
    </div>
  );
}
