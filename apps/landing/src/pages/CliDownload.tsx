import { useState } from "react";
import { Link } from "react-router-dom";
import { Button } from "../components";
import { DOWNLOADS, GITHUB_REPO, LANDING_URL } from "../config";

// Every install one-liner in one place. Copy-to-clipboard is the primary
// interaction; we don't try to auto-select a tab for the visitor's OS —
// the previous "your OS" pill added visual noise for a saving of one
// glance. All three commands are equally visible.

interface Option {
  os: string;
  method: string;
  command: string;
  hint: string;
}

const OPTIONS: Option[] = [
  {
    os: "macOS",
    method: "Homebrew",
    command: "brew install taxcollector23/trace/trace",
    hint: "Requires Homebrew. Tap is auto-added on first install.",
  },
  {
    os: "macOS · Linux",
    method: "curl",
    command: `curl -fsSL ${LANDING_URL}/install.sh | sh`,
    hint: "Downloads the right binary for your arch to ~/.trace/bin/trc and prints PATH instructions.",
  },
  {
    os: "Windows",
    method: "irm | iex",
    command: `irm ${LANDING_URL}/install.ps1 | iex`,
    hint: "Run in PowerShell. Installs to %USERPROFILE%\\.trace\\bin and updates your user PATH.",
  },
  {
    os: "Any · npm",
    method: "npm",
    command: "npm install -g trace-dev",
    hint: "Cross-platform. Pulls the matching binary from GitHub Releases and puts `trc` on your PATH.",
  },
];

const AFTER_INSTALL = [
  {
    step: "1. Confirm the install",
    code: "trc --version",
    body: "You should see \"Trace 1.2\" or later.",
  },
  {
    step: "2. Start the daemon",
    code: "trc daemon start",
    body: "The daemon listens on 127.0.0.1 only. It holds the session store, the dashboard, and the review pipeline.",
  },
  {
    step: "3. Wire up every agent you use",
    code: "trc integrations install all",
    body: "Idempotently patches Claude Code settings, Cursor MCP config, and Windsurf MCP config. Timestamped backups on first write.",
  },
  {
    step: "4. Watch a real session",
    code: "trc run \"claude fix the bug in src/api.py\"",
    body: "Or launch your agent normally after step 3 — the hooks report back automatically. Open the dashboard with `trc dashboard`.",
  },
];

function CopyBlock({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="group relative">
      <pre className="overflow-x-auto rounded-xl border border-border bg-[#0d0d10] px-5 py-4 font-mono text-sm leading-relaxed text-white">
        <span className="select-none text-white/40">$ </span>
        {text}
      </pre>
      <button
        onClick={() => {
          navigator.clipboard.writeText(text);
          setCopied(true);
          setTimeout(() => setCopied(false), 1400);
        }}
        className="absolute right-3 top-3 rounded-md border border-white/20 bg-white/10 px-2.5 py-1 text-[11px] font-medium text-white opacity-0 transition-opacity hover:bg-white/20 group-hover:opacity-100"
      >
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}

export default function CliDownload() {
  return (
    <div className="py-20">
      <Link to="/" className="text-sm text-text-dim hover:text-text">
        ← Back
      </Link>

      <div className="mt-10 max-w-[720px]">
        <h1 className="font-serif text-4xl text-text">Install the Trace CLI</h1>
        <p className="mt-4 text-lg text-text-dim">
          The full <code className="rounded bg-black/5 px-1.5 py-0.5 text-sm">trc</code> binary
          — daemon, dashboard, hook installer, PR ratification, everything. One command per
          platform.
        </p>
      </div>

      <div className="mt-14 grid grid-cols-1 gap-6">
        {OPTIONS.map((opt) => (
          <div key={opt.method} className="rounded-2xl border border-border bg-white p-8">
            <div className="mb-4 flex items-baseline justify-between">
              <div>
                <div className="text-sm uppercase tracking-wide text-text-dim">{opt.os}</div>
                <div className="mt-1 font-serif text-xl text-text">{opt.method}</div>
              </div>
            </div>
            <CopyBlock text={opt.command} />
            <p className="mt-3 text-sm text-text-dim">{opt.hint}</p>
          </div>
        ))}
      </div>

      <div className="mt-20 max-w-[720px]">
        <h2 className="font-serif text-2xl text-text">After it&apos;s installed</h2>
        <div className="mt-8 space-y-8">
          {AFTER_INSTALL.map((s) => (
            <div key={s.step}>
              <div className="text-sm font-semibold text-text">{s.step}</div>
              <div className="mt-2">
                <CopyBlock text={s.code} />
              </div>
              <p className="mt-2 text-sm text-text-dim">{s.body}</p>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-20 rounded-2xl border border-border bg-white/50 p-8 text-center">
        <div className="font-serif text-xl text-text">Prefer the desktop app?</div>
        <p className="mx-auto mt-2 max-w-[520px] text-sm text-text-dim">
          Same daemon, same dashboard, no terminal required. macOS DMG, Windows installer, Linux
          .deb and AppImage.
        </p>
        <div className="mt-6 flex flex-wrap items-center justify-center gap-4">
          <Button to="/download">Download desktop app</Button>
          <Button variant="secondary" href={GITHUB_REPO} target="_blank" rel="noreferrer">
            View source
          </Button>
        </div>
        <p className="mt-6 text-xs text-text-dim">
          All release binaries and checksums live on the{" "}
          <a
            href={DOWNLOADS.releases}
            target="_blank"
            rel="noreferrer"
            className="text-brand hover:text-brand-dim"
          >
            GitHub releases page
          </a>
          .
        </p>
      </div>
    </div>
  );
}
