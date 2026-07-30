# Trace — Cursor MCP integration

An MCP server that exposes Trace operations as tools, backed by the local
daemon. Dependency-free (Node ≥ 18).

## Tools

- `trace_start_run`
- `trace_end_run`
- `trace_record_event`
- `trace_get_recent_runs`
- `trace_get_patch_summary`
- `trace_check_command`
- `trace_get_rollback_options`

## Configure in Cursor

Add to your Cursor MCP config (`~/.cursor/mcp.json` or the MCP settings UI):

```json
{
  "mcpServers": {
    "trace": {
      "command": "node",
      "args": ["/absolute/path/to/trace/integrations/cursor/src/index.js"]
    }
  }
}
```

The server reads the daemon port from `~/.trace/daemon.json` and talks only
to `http://127.0.0.1:<port>`.

## Honest limitation

Trace can observe project file changes and Git diffs. **Full command
blocking requires supported integration points or running commands through
`trace`.** The `trace_check_command` tool returns a guard decision, but
enforcing it is up to the client.
