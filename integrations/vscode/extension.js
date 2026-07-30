// Trace VS Code extension — a lightweight bridge to the local daemon.
//
// It does not duplicate the dashboard. It shows recent runs, opens the
// dashboard, runs the current command through `trace`, and can wrap a
// GitHub Copilot editing session as a real Trace run: every file you save
// while the session is active is recorded as a file-change event and
// scanned for obvious secrets, so Copilot edits show up in Patch Review and
// Risk Center like any other agent run. It only talks to
// http://127.0.0.1:<port> using the port from ~/.trace/daemon.json.

const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");

function daemonBaseUrl() {
  try {
    const statePath = path.join(os.homedir(), ".trace", "daemon.json");
    const state = JSON.parse(fs.readFileSync(statePath, "utf8"));
    if (state && state.port) return `http://127.0.0.1:${state.port}`;
  } catch (_) {
    /* daemon not running */
  }
  return null;
}

async function apiGet(base, p) {
  const res = await fetch(`${base}${p}`);
  if (!res.ok) throw new Error(`GET ${p} failed: ${res.status}`);
  return res.json();
}

async function apiPost(base, p, body) {
  const res = await fetch(`${base}${p}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) throw new Error(`POST ${p} failed: ${res.status}`);
  return res.json();
}

async function fetchRuns() {
  const base = daemonBaseUrl();
  if (!base) return [];
  try {
    return await apiGet(base, "/api/runs?limit=25");
  } catch (_) {
    return [];
  }
}

/// Crude, client-side secret patterns — enough to catch obvious leaks
/// (cloud keys, private key blocks, generic long bearer-style tokens)
/// without pulling in a dependency. Mirrors the spirit of the daemon's own
/// secret scanner, not a copy of it.
const SECRET_PATTERNS = [
  { type: "aws_access_key", re: /AKIA[0-9A-Z]{16}/g },
  { type: "private_key", re: /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/g },
  { type: "generic_api_key", re: /\b(?:api[_-]?key|secret|token)\b\s*[:=]\s*["'][A-Za-z0-9_\-]{20,}["']/gi },
  { type: "slack_token", re: /xox[baprs]-[0-9A-Za-z-]{10,}/g },
];

function scanForSecrets(text) {
  const found = [];
  for (const { type, re } of SECRET_PATTERNS) {
    const matches = text.match(re);
    if (matches) {
      for (const m of matches) {
        found.push({ type, redacted: m.slice(0, 4) + "…" + m.slice(-4) });
      }
    }
  }
  return found;
}

class RunsProvider {
  constructor() {
    this._onDidChange = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._onDidChange.event;
    this.runs = [];
  }
  refresh() {
    fetchRuns().then((runs) => {
      this.runs = runs;
      this._onDidChange.fire();
    });
  }
  getTreeItem(item) {
    return item;
  }
  getChildren() {
    if (!daemonBaseUrl()) {
      const item = new vscode.TreeItem("Daemon not running — run `trace dashboard`");
      item.iconPath = new vscode.ThemeIcon("debug-disconnect");
      return [item];
    }
    if (this.runs.length === 0) {
      return [new vscode.TreeItem("No runs yet")];
    }
    return this.runs.map((r) => {
      const label = `${r.status.toUpperCase()} · ${r.command}`;
      const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
      item.description = `${r.files_changed} files · ${r.secret_warnings} secrets`;
      item.tooltip = `${r.project_name}\n${r.command}`;
      item.iconPath = new vscode.ThemeIcon(
        r.status === "failed" || r.status === "blocked" ? "error" : "pass"
      );
      return item;
    });
  }
}

/// Wraps GitHub Copilot (or any editor-driven) sessions as a real Trace run.
/// There's no Copilot API to hook directly, so this attributes activity by
/// time window instead: while a session is active, every file save in the
/// workspace is recorded as a file-change event on that run and scanned for
/// obvious secrets, so it shows up in Patch Review / Risk Center like any
/// CLI-wrapped agent.
class CopilotSession {
  constructor(statusBarItem) {
    this.runId = null;
    this.statusBarItem = statusBarItem;
  }

  get active() {
    return this.runId !== null;
  }

  async start() {
    const base = daemonBaseUrl();
    if (!base) {
      vscode.window.showErrorMessage("Trace daemon isn't running. Run `trace dashboard` first.");
      return;
    }
    const folder = vscode.workspace.workspaceFolders?.[0];
    if (!folder) {
      vscode.window.showErrorMessage("Open a folder to start a Trace session.");
      return;
    }
    const projectPath = folder.uri.fsPath;
    const projects = await apiGet(base, "/api/projects").catch(() => []);
    const project = projects.find((p) => p.path === projectPath);
    if (!project) {
      vscode.window.showErrorMessage(
        `This folder isn't a Trace project yet. Run \`trace init\` in ${projectPath} first.`
      );
      return;
    }
    const run = await apiPost(base, "/api/runs", {
      project_id: project.id,
      command: "GitHub Copilot session",
      agent_name: "copilot",
      user_prompt: null,
      starting_commit: null,
    });
    this.runId = run.id;
    this.statusBarItem.text = "$(record) Trace: Copilot session";
    this.statusBarItem.show();
    vscode.window.showInformationMessage("Trace is now recording file changes from this Copilot session.");
  }

  async end() {
    if (!this.active) return;
    const base = daemonBaseUrl();
    try {
      if (base) {
        await apiPost(base, `/api/runs/${this.runId}/finish`, { status: "completed" });
      }
    } finally {
      this.runId = null;
      this.statusBarItem.hide();
    }
  }

  async onSave(document) {
    if (!this.active) return;
    const base = daemonBaseUrl();
    if (!base) return;
    const relPath = vscode.workspace.asRelativePath(document.uri, false);

    await apiPost(base, `/api/runs/${this.runId}/events`, {
      type: "file_saved",
      message: relPath,
      metadata_json: null,
    }).catch(() => {});

    const secrets = scanForSecrets(document.getText());
    for (const s of secrets) {
      await apiPost(base, `/api/runs/${this.runId}/secrets`, {
        file_path: relPath,
        secret_type: s.type,
        redacted_value: s.redacted,
        action_taken: "warned",
      }).catch(() => {});
    }
    if (secrets.length > 0) {
      vscode.window.showWarningMessage(
        `Trace: possible ${secrets[0].type.replace(/_/g, " ")} in ${relPath} — check Risk Center.`
      );
    }
  }
}

function activate(context) {
  const provider = new RunsProvider();
  provider.refresh();

  const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
  const session = new CopilotSession(statusBarItem);

  context.subscriptions.push(
    statusBarItem,
    vscode.window.registerTreeDataProvider("trace.runs", provider),
    vscode.commands.registerCommand("trace.refresh", () => provider.refresh()),
    vscode.commands.registerCommand("trace.openDashboard", () => {
      const base = daemonBaseUrl();
      if (base) {
        vscode.env.openExternal(vscode.Uri.parse(base));
      } else {
        const term = vscode.window.createTerminal("Trace");
        term.show();
        term.sendText("trace dashboard");
      }
    }),
    vscode.commands.registerCommand("trace.runCommand", async () => {
      const command = await vscode.window.showInputBox({
        prompt: "Command to run through Trace",
        placeHolder: 'e.g. claude "fix the login bug"',
      });
      if (!command) return;
      const term = vscode.window.createTerminal("Trace");
      term.show();
      term.sendText(`trace run ${JSON.stringify(command)}`);
    }),
    vscode.commands.registerCommand("trace.startCopilotSession", () => session.start()),
    vscode.commands.registerCommand("trace.endCopilotSession", () => session.end()),
    vscode.workspace.onDidSaveTextDocument((doc) => session.onSave(doc)),
    { dispose: () => session.end() }
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
