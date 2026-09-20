# Trace + Cursor

Connect Cursor with one command:

```bash
npm install -g trace-agent-cli
trc integrations install cursor
```

The installer writes the MCP server and a shared hook to `~/.trace`, patches
`~/.cursor/mcp.json` and `~/.cursor/hooks.json`, preserves other entries, and
backs up files before changing them.

The hook starts and finishes a Trace run with each Cursor session, checks every
shell command before execution, and records shell completions and file edits.
The guard fails closed if the local Trace daemon is unavailable.

Restart Cursor if it does not reload the settings automatically. Verify with:

```bash
trc integrations status
trc dashboard
```
