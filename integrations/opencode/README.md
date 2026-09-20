# Trace + OpenCode

Connect OpenCode with one command:

```bash
npm install -g trace-agent-cli
trc integrations install opencode
```

The installer registers Trace's local MCP server and a plugin in OpenCode's
config. The plugin supports OpenCode V1 and V2: it creates a run when the first
tool is used, checks Bash commands before they run, records completed tools, and
captures the working tree after edits.

The command guard fails closed when Trace's local daemon is unavailable. File
review is after the edit, so use the dashboard undo point or `trc rollback` if
you want to remove a change.

Restart OpenCode after installation, then verify:

```bash
trc integrations status
trc dashboard
```
