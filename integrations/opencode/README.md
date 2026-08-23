# Trace × opencode

[opencode](https://github.com/sst/opencode) is an open-source terminal coding
agent that speaks the Model Context Protocol (MCP). Trace connects to it as a
**local MCP server** — the same daemon-backed server the Cursor and Windsurf
integrations use — so opencode can classify commands, record runs, and read
patch/rollback state through Trace while everything stays local.

## Install

```bash
trc integrations install opencode   # or: trc integrations install all
```

This:

1. Writes the MCP server to `~/.trace/integrations/opencode/index.js`.
2. Idempotently patches your global opencode config
   (`~/.config/opencode/opencode.json`, XDG-aware) with a `trace` MCP entry,
   backing the file up first and preserving everything else you have there.

The resulting entry:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "trace": {
      "type": "local",
      "command": ["node", "/Users/you/.trace/integrations/opencode/index.js"],
      "enabled": true
    }
  }
}
```

opencode merges every config file under `~/.config/opencode/`, so this coexists
with any other MCP servers you already run. Restart opencode to load it.

## Verify

```bash
trc integrations status   # opencode should read "connected"
```

## Tools exposed

`trace_check_command`, `trace_get_recent_runs`, `trace_start_run`,
`trace_end_run`, `trace_record_event`, `trace_get_patch_summary`,
`trace_get_rollback_options`.

The server proxies to the local daemon on `127.0.0.1`; start it with
`trc daemon start`. If the daemon isn't running, tool calls fail closed and
opencode simply proceeds without Trace.
