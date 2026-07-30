// TraceGuard VS Code extension — a lightweight bridge to the local daemon.
//
// It does not duplicate the dashboard. It shows recent runs, opens the
// dashboard, and runs the current command through `trg`. It only talks to
// http://127.0.0.1:<port> using the port from ~/.traceguard/daemon.json.

const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");

function daemonBaseUrl() {
  try {
    const statePath = path.join(os.homedir(), ".traceguard", "daemon.json");
    const state = JSON.parse(fs.readFileSync(statePath, "utf8"));
    if (state && state.port) return `http://127.0.0.1:${state.port}`;
  } catch (_) {
    /* daemon not running */
  }
  return null;
}

async function fetchRuns() {
  const base = daemonBaseUrl();
  if (!base) return [];
  try {
    const res = await fetch(`${base}/api/runs?limit=25`);
    if (!res.ok) return [];
    return await res.json();
  } catch (_) {
    return [];
  }
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
      const item = new vscode.TreeItem("Daemon not running — run `trg dashboard`");
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

function activate(context) {
  const provider = new RunsProvider();
  provider.refresh();
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider("traceguard.runs", provider),
    vscode.commands.registerCommand("traceguard.refresh", () => provider.refresh()),
    vscode.commands.registerCommand("traceguard.openDashboard", () => {
      const base = daemonBaseUrl();
      if (base) {
        vscode.env.openExternal(vscode.Uri.parse(base));
      } else {
        const term = vscode.window.createTerminal("TraceGuard");
        term.show();
        term.sendText("trg dashboard");
      }
    }),
    vscode.commands.registerCommand("traceguard.runCommand", async () => {
      const command = await vscode.window.showInputBox({
        prompt: "Command to run through TraceGuard",
        placeHolder: 'e.g. claude "fix the login bug"',
      });
      if (!command) return;
      const term = vscode.window.createTerminal("TraceGuard");
      term.show();
      term.sendText(`trg run ${JSON.stringify(command)}`);
    })
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
