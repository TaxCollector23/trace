import type { Icon } from "@phosphor-icons/react";
import { PlugsConnected, Terminal, WindowsLogo, GithubLogo, Code, GearSix } from "@phosphor-icons/react";
import { Reveal } from "./components";

type Status = "Available" | "Adapter-ready" | "Planned";

const CONNECTORS: { name: string; note: string; status: Status; icon: Icon }[] = [
  { name: "Claude Code", note: "Wrapper + hooks adapter", status: "Available", icon: Terminal },
  { name: "Codex CLI", note: "Wrapper adapter", status: "Available", icon: Terminal },
  { name: "Aider", note: "Wrapper adapter", status: "Available", icon: Terminal },
  { name: "OpenCode", note: "Wrapper adapter", status: "Available", icon: Terminal },
  { name: "Gemini CLI", note: "Wrapper adapter", status: "Available", icon: Terminal },
  { name: "Generic terminal agent", note: "trace run <command>", status: "Available", icon: GearSix },
  { name: "Cursor", note: "MCP server integration", status: "Adapter-ready", icon: Code },
  { name: "VS Code", note: "Extension", status: "Adapter-ready", icon: Code },
  { name: "GitHub Actions", note: "CI summary workflow", status: "Adapter-ready", icon: GithubLogo },
  { name: "GitHub App / PR checks", note: "Skeleton", status: "Planned", icon: WindowsLogo },
];

const statusStyle: Record<Status, string> = {
  Available: "border-good/30 bg-good/10 text-good",
  "Adapter-ready": "border-brand/30 bg-brand/10 text-brand",
  Planned: "border-border-strong bg-surface-2 text-text-dimmer",
};

export default function WorksEverywhere() {
  return (
    <div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {CONNECTORS.map((c, i) => (
          <Reveal key={c.name} delay={i * 0.03}>
            <div className="flex h-full flex-col justify-between gap-4 rounded-md border border-border bg-surface p-4">
              <div className="flex items-start justify-between gap-2">
                <div className="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-surface-2 text-text-dim">
                  <c.icon size={17} weight="bold" />
                </div>
                <span className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium ${statusStyle[c.status]}`}>
                  {c.status}
                </span>
              </div>
              <div>
                <div className="text-sm font-semibold">{c.name}</div>
                <div className="mt-0.5 text-xs text-text-dim">{c.note}</div>
              </div>
            </div>
          </Reveal>
        ))}
      </div>
      <div className="mt-6 flex items-start gap-2 text-xs text-text-dimmer">
        <PlugsConnected size={14} className="mt-0.5 shrink-0" />
        <span>
          Every adapter implements the same interface — session lifecycle, file
          watch, command capture, Git status. New tools plug in without
          touching the core recorder.
        </span>
      </div>
    </div>
  );
}
