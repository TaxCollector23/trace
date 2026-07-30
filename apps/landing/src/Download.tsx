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

/** Best-effort OS detection from the browser, for the recommended tab. */
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
    <section id="download" className="section">
      <div className="kicker">Install</div>
      <h2>Download the CLI</h2>
      <p className="lede">
        Install TraceGuard, initialize your repo, run your agent through{" "}
        <code>trg</code>, and open the local dashboard. One binary, two commands:{" "}
        <code>trg</code> and <code>traceguard</code>. Detected{" "}
        <b>{INSTALL[os].name}</b> — recommended below.
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
          <div className="dl-grid">
            <div>
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
            </div>

            <div className="dl-after">
              <div className="dl-after-title">Then record your first run</div>
              <Cmd>trg init</Cmd>
              <Cmd>trg run claude</Cmd>
              <Cmd>trg dashboard</Cmd>
              <p className="muted dl-after-note">
                The dashboard opens at <code>http://127.0.0.1:&lt;port&gt;</code>{" "}
                — local only.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
