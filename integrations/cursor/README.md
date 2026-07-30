# TraceGuard — Cursor MCP integration

An MCP server that exposes TraceGuard operations as tools, backed by the local
daemon. Dependency-free (Node ≥ 18).

## Tools

- `traceguard_start_run`
- `traceguard_end_run`
- `traceguard_record_event`
- `traceguard_get_recent_runs`
- `traceguard_get_patch_summary`
- `traceguard_check_command`
- `traceguard_get_rollback_options`

## Configure in Cursor

Add to your Cursor MCP config (`~/.cursor/mcp.json` or the MCP settings UI):

```json
{
  "mcpServers": {
    "traceguard": {
      "command": "node",
      "args": ["/absolute/path/to/TraceGuard/integrations/cursor/src/index.js"]
    }
  }
}
```

The server reads the daemon port from `~/.traceguard/daemon.json` and talks only
to `http://127.0.0.1:<port>`.

## Honest limitation

TraceGuard can observe project file changes and Git diffs. **Full command
blocking requires supported integration points or running commands through
`trg`.** The `traceguard_check_command` tool returns a guard decision, but
enforcing it is up to the client.
