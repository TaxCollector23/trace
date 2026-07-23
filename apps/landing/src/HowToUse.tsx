import { Reveal, Cmd } from "./components";

interface Guide {
  name: string;
  logo: string;
  kind: "cli" | "gui";
  steps: string[];
  command?: string;
  snippet?: string;
}

const GUIDES: Guide[] = [
  {
    name: "Claude Code",
    logo: "/logos/claude.png",
    kind: "cli",
    steps: [
      "Run Claude Code through Trace instead of calling it directly.",
      "Trace launches Claude Code as a subprocess and watches its session output for file edits, shell commands, and token usage.",
      "Open the dashboard any time to see the run's diff, cost, and any blocked commands.",
    ],
    command: 'trace run "claude fix the login bug"',
  },
  {
    name: "Codex CLI",
    logo: "/logos/codex.png",
    kind: "cli",
    steps: [
      "Same pattern as Claude Code — wrap the codex command with trace run.",
      "Trace parses Codex's event stream to track every file change and command it executes.",
      "Nothing to configure inside Codex itself.",
    ],
    command: 'trace run "codex refactor the auth middleware"',
  },
  {
    name: "Cursor",
    logo: "/logos/cursor.png",
    kind: "gui",
    steps: [
      "Cursor is GUI-only, so it connects to Trace as an MCP server instead of a wrapped command.",
      "Point Cursor's MCP config at Trace's local server — it exposes run, patch, rollback, and command-check tools, backed by your local daemon.",
      "Cursor's agent then calls Trace directly, so every edit shows up in the dashboard as it happens.",
    ],
    snippet: '{ "mcpServers": { "trace": { "command": "node", "args": ["integrations/cursor/src/index.js"] } } }',
  },
  {
    name: "OpenCode",
    logo: "/logos/opencode.png",
    kind: "cli",
    steps: [
      "Wrap OpenCode the same way as Claude Code and Codex.",
      "Trace normalizes OpenCode's event format into the same run timeline as every other agent.",
    ],
    command: 'trace run "opencode add pagination to /api/users"',
  },
  {
    name: "GitHub Copilot",
    logo: "/logos/copilot.png",
    kind: "gui",
    steps: [
      "Copilot Chat runs inside your editor, so Trace can't wrap it as a subprocess the way it does CLI agents.",
      "Install the Trace VS Code extension — it bridges to your local daemon, shows recent runs in a sidebar, and flags risky modified files as Copilot edits them.",
      "Coarser than the CLI adapters today: file changes and Git diffs only, no per-command risk classification or token cost.",
    ],
  },
];

export default function HowToUse() {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
      {GUIDES.map((g, i) => (
        <Reveal key={g.name} delay={i * 0.03}>
          <div className="h-full rounded-lg bg-surface-2 p-5">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-surface">
                <img src={g.logo} alt={`${g.name} logo`} className="h-full w-full object-cover" />
              </div>
              <span className="text-sm font-medium text-text">{g.name}</span>
              <span className="ml-auto rounded-full bg-surface px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-text-dimmer">
                {g.kind === "cli" ? "CLI" : "GUI"}
              </span>
            </div>

            <ul className="mt-4 space-y-2">
              {g.steps.map((s, j) => (
                <li key={j} className="flex gap-2 text-sm text-text-dim">
                  <span className="mt-0.5 shrink-0 text-brand">→</span>
                  <span>{s}</span>
                </li>
              ))}
            </ul>

            {g.command && (
              <div className="mt-4">
                <Cmd>{g.command}</Cmd>
              </div>
            )}
            {g.snippet && (
              <div className="mt-4 overflow-x-auto rounded bg-black/40 px-4 py-2.5">
                <code className="whitespace-pre text-xs text-text-dim">{g.snippet}</code>
              </div>
            )}
          </div>
        </Reveal>
      ))}
    </div>
  );
}
