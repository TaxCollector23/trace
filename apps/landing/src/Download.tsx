import { useState } from "react";
import { Link } from "react-router-dom";
import { Cmd, Section } from "./components";

type OS = "macos" | "windows" | "linux";

const INSTALL: Record<OS, { name: string; command: string }> = {
  macos: { name: "macOS", command: "curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh" },
  windows: { name: "Windows", command: "npm install -g tracedev" },
  linux: { name: "Linux", command: "curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh" },
};

function detectOS(): OS {
  if (typeof navigator === "undefined") return "macos";
  const s = `${navigator.userAgent} ${navigator.platform}`.toLowerCase();
  if (s.includes("win")) return "windows";
  if (s.includes("linux") || s.includes("x11")) return "linux";
  return "macos";
}

export default function Download() {
  const [os, setOs] = useState<OS>(detectOS);
  const active = INSTALL[os];

  return (
    <Section id="download">
      <div className="flex items-start justify-between gap-6 flex-wrap">
        <div>
          <h2 className="text-2xl font-semibold">Install the CLI</h2>
        </div>
        <Link
          to="/desktop"
          className="rounded bg-surface px-4 py-2 text-sm font-medium text-text-dim hover:text-text hover:bg-surface-2"
        >
          Or download the desktop app for macOS
        </Link>
      </div>

      <div className="mt-6 flex gap-6">
        {(Object.keys(INSTALL) as OS[]).map((key) => (
          <button
            key={key}
            onClick={() => setOs(key)}
            className={`py-2 text-sm font-medium ${
              os === key ? "text-brand" : "text-text-dim hover:text-text"
            }`}
          >
            {INSTALL[key].name}
          </button>
        ))}
      </div>

      <div className="mt-4 max-w-[520px]">
        <Cmd>{active.command}</Cmd>
      </div>
    </Section>
  );
}
