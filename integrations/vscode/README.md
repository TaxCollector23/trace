# Trace — VS Code extension

A lightweight bridge to your local Trace daemon. It does **not** duplicate
the dashboard.

## Features

- **Recent Runs** view in the activity bar
- **Trace: Open Dashboard** command
- **Trace: Run Command through trace** command
- Status icons for failed/blocked runs

## Develop / run

```bash
cd integrations/vscode
# Plain JS extension — no build step.
code .
# Press F5 to launch an Extension Development Host.
```

The extension reads the daemon port from `~/.trace/daemon.json` and talks
only to `http://127.0.0.1:<port>`. If the daemon is not running it offers to run
`trace dashboard` for you. It never connects to any remote service.
