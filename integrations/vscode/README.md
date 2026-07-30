# TraceGuard — VS Code extension

A lightweight bridge to your local TraceGuard daemon. It does **not** duplicate
the dashboard.

## Features

- **Recent Runs** view in the activity bar
- **TraceGuard: Open Dashboard** command
- **TraceGuard: Run Command through trg** command
- Status icons for failed/blocked runs

## Develop / run

```bash
cd integrations/vscode
# Plain JS extension — no build step.
code .
# Press F5 to launch an Extension Development Host.
```

The extension reads the daemon port from `~/.traceguard/daemon.json` and talks
only to `http://127.0.0.1:<port>`. If the daemon is not running it offers to run
`trg dashboard` for you. It never connects to any remote service.
