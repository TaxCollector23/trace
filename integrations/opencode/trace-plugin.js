// Trace guard plugin for OpenCode.
//
// OpenCode calls `tool.execute.before` right before it runs any tool. This
// plugin intercepts the `bash` tool, classifies the command with the Trace
// daemon's deterministic guard, and THROWS on a "block" decision, which
// cancels the command before it runs. This is the enforced counterpart to
// Trace's OpenCode MCP tools (which are advisory).
//
// It FAILS OPEN: if the daemon is not running or the request errors, the
// command is allowed, so Trace can never wedge OpenCode.
//
// Auto-loaded from ~/.config/opencode/plugin/ at startup. No config needed.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

function daemonBase() {
  try {
    const raw = fs.readFileSync(path.join(os.homedir(), ".trace", "daemon.json"), "utf8");
    const state = JSON.parse(raw);
    const port = process.env.TRACE_DAEMON_PORT || (state && state.port);
    if (port) return `http://127.0.0.1:${port}`;
  } catch {
    // no daemon file -> fail open
  }
  return null;
}

export const TraceGuard = async () => ({
  "tool.execute.before": async (input, output) => {
    if (input.tool !== "bash") return;
    const command = output && output.args && output.args.command;
    if (!command) return;

    const base = daemonBase();
    if (!base) return; // daemon down -> allow

    let data;
    try {
      const res = await fetch(`${base}/api/check-command`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ command }),
        signal: AbortSignal.timeout(3000),
      });
      if (!res.ok) return;
      data = await res.json();
    } catch {
      return; // daemon unreachable -> fail open
    }

    if (data && data.decision === "block") {
      throw new Error(`Trace blocked this command: ${data.reason || "matched a high-risk rule"}`);
    }
  },
});
