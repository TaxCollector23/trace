import { useState } from "react";
import { Cmd } from "./components";
import { RAW_BASE } from "./config";

type OS = "macos" | "linux" | "windows";

interface InstallOption {
  label: string;
  recommended?: boolean;
  commands: string[];
  note?: string;
}

const INSTALL: Record<OS, { name: string; icon: string; options: InstallOption[] }> = {
  macos: {
    name: "macOS",
    icon: "",
    options: [
      {
        label: "Homebrew",
        recommended: true,
        commands: ["brew tap TaxCollector23/traceguard", "brew install traceguard"],
      },
      { label: "curl", commands: [`curl -fsSL ${RAW_BASE}/scripts/install.sh | sh`] },
      { label: "npm", commands: ["npm install -g traceguard"] },
    ],
  },
  linux: {
    name: "Linux",
    icon: "🐧",
    options: [
      {
        label: "curl",
        recommended: true,
        commands: [`curl -fsSL ${RAW_BASE}/scripts/install.sh | sh`],
      },
      { label: "Homebrew", commands: ["brew tap TaxCollector23/traceguard", "brew install traceguard"] },
      { label: "npm", commands: ["npm install -g traceguard"] },
    ],
  },
  windows: {
    name: "Windows",
    icon: "⊞",
    options: [
      {
        label: "PowerShell",
        recommended: true,
        commands: [`irm ${RAW_BASE}/scripts/install.ps1 | iex`],
      },
      { label: "npm", commands: ["npm install -g traceguard"] },
    ],
  },
};

export default function Download() {
  const [os, setOs] = useState<OS>("macos");
  const active = INSTALL[os];

  return (
    <section id="download" className="section">
      <div className="kicker">Install</div>
      <h2>Download the CLI</h2>
      <p className="muted">
        One binary, two commands: <code>trg</code> and <code>traceguard</code>.
        Pick your platform.
      </p>

      <div className="dl">
        <div className="dl-tabs" role="tablist">
          {(Object.keys(INSTALL) as OS[]).map((key) => (
            <button
              key={key}
              role="tab"
              aria-selected={os === key}
              className={`dl-tab ${os === key ? "active" : ""}`}
              onClick={() => setOs(key)}
            >
              {INSTALL[key].icon && <span className="dl-ico">{INSTALL[key].icon}</span>}
              {INSTALL[key].name}
            </button>
          ))}
        </div>

        <div className="dl-body">
          {active.options.map((opt) => (
            <div className="dl-opt" key={opt.label}>
              <div className="dl-opt-head">
                <span className="dl-opt-label">{opt.label}</span>
                {opt.recommended && <span className="dl-badge">Recommended</span>}
              </div>
              {opt.commands.map((c) => (
                <Cmd key={c}>{c}</Cmd>
              ))}
            </div>
          ))}

          <div className="dl-after">
            <div className="dl-after-title">Then verify and record your first run</div>
            <Cmd>trg --help</Cmd>
            <Cmd>trg init</Cmd>
            <Cmd>trg run claude</Cmd>
            <Cmd>trg dashboard</Cmd>
            <p className="muted dl-after-note">
              The dashboard opens at <code>http://127.0.0.1:&lt;port&gt;</code> —
              local only.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
