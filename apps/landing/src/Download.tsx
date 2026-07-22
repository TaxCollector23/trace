import { useState } from "react";
import { Cmd, Section } from "./components";
import { GITHUB_REPO, RAW_BASE } from "./config";

type OS = "macos" | "linux" | "windows";

interface InstallOption {
  label: string;
  recommended?: boolean;
  commands: string[];
}

// npm is the default everywhere: it needs nothing pre-installed beyond
// Node, which most developers already have — unlike Homebrew, which is
// macOS/Linuxbrew-only and not something to assume.
const INSTALL: Record<OS, { name: string; options: InstallOption[] }> = {
  macos: {
    name: "macOS",
    options: [
      { label: "npm", recommended: true, commands: ["npm install -g trace"] },
      { label: "Homebrew", commands: ["brew tap TaxCollector23/tap", "brew install trace"] },
      { label: "curl", commands: [`curl -fsSL ${RAW_BASE}/scripts/install.sh | sh`] },
    ],
  },
  linux: {
    name: "Linux",
    options: [
      { label: "npm", recommended: true, commands: ["npm install -g trace"] },
      { label: "curl", commands: [`curl -fsSL ${RAW_BASE}/scripts/install.sh | sh`] },
      { label: "Homebrew", commands: ["brew tap TaxCollector23/tap", "brew install trace"] },
    ],
  },
  windows: {
    name: "Windows",
    options: [
      { label: "npm", recommended: true, commands: ["npm install -g trace"] },
      { label: "PowerShell", commands: [`irm ${RAW_BASE}/scripts/install.ps1 | iex`] },
    ],
  },
};

function detectOS(): OS {
  if (typeof navigator === "undefined") return "macos";
  const s = `${navigator.userAgent} ${navigator.platform}`.toLowerCase();
  if (s.includes("win")) return "windows";
  if (s.includes("mac")) return "macos";
  if (s.includes("linux") || s.includes("x11")) return "linux";
  return "macos";
}

export default function Download() {
  const [os, setOs] = useState<OS>(detectOS);
  const active = INSTALL[os];

  return (
    <Section
      id="download"
      title="Download the CLI"
      lede={
        <>
          Install Trace, initialize your repo, run your agent through{" "}
          <code className="text-text">trace</code>, and open the local
          dashboard. One binary: <code className="text-text">trace</code>.
          Detected <span className="text-text">{active.name}</span> — recommended below.
        </>
      }
    >
      <div className="flex gap-6 border-b border-border">
        {(Object.keys(INSTALL) as OS[]).map((key) => (
          <button
            key={key}
            onClick={() => setOs(key)}
            className={`-mb-px border-b-2 py-2.5 text-sm font-medium ${
              os === key ? "border-brand text-text" : "border-transparent text-text-dim hover:text-text"
            }`}
          >
            {INSTALL[key].name}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-1 gap-8 py-6 md:grid-cols-2">
        <div className="space-y-5">
          {active.options.map((opt) => (
            <div key={opt.label}>
              <div className="mb-1.5 flex items-center gap-2">
                <span className="text-sm font-semibold">{opt.label}</span>
                {opt.recommended && (
                  <span className="rounded-sm border border-brand-dim px-1.5 py-0.5 text-[11px] font-medium text-brand">
                    Recommended
                  </span>
                )}
              </div>
              {opt.commands.map((c) => (
                <Cmd key={c}>{c}</Cmd>
              ))}
            </div>
          ))}
        </div>

        <div>
          <div className="mb-2.5 text-xs uppercase tracking-wide text-text-dim">
            Then record your first run
          </div>
          <Cmd>trace init</Cmd>
          <Cmd>trace run claude</Cmd>
          <Cmd>trace dashboard</Cmd>
          <p className="mt-2.5 text-sm text-text-dim">
            The dashboard opens at <code className="text-text">http://127.0.0.1:&lt;port&gt;</code> — local only.
          </p>
        </div>
      </div>

      <p className="mt-2 text-sm text-text-dim">
        Prefer a native window over a browser tab?{" "}
        <a
          href={`${GITHUB_REPO}/releases/latest`}
          target="_blank"
          rel="noreferrer"
          className="font-medium text-brand hover:text-brand-dim"
        >
          Download the desktop app for macOS
        </a>{" "}
        — same dashboard, same daemon, in its own window. Unsigned for now, so
        macOS will ask you to confirm the first launch.
      </p>
    </Section>
  );
}
