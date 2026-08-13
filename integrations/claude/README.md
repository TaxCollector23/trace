# Trace — Claude Code integration

## Wrapper (recommended, works everywhere)

```bash
trace run claude
trace run "claude fix the login bug"
```

This is what makes the hooks below actually able to find your run — it sets
`TRACE_RUN_ID` and `TRACE_DAEMON_PORT` in the environment of the `claude`
process it launches, and every hook `claude` fires inherits them.

## Hooks adapter (finer-grained, live intervention)

`trace-hook.sh` handles two Claude Code hook events:

- **`PreToolUse` (Bash)** — asks the local daemon to classify the command
  with the rule-based guard and blocks it (exit code 2) on a `block`
  decision. Unchanged from before.
- **`PostToolUse` (Edit / Write / MultiEdit / NotebookEdit)** — sends the
  edit to Trace's live review path, where the deterministic policy engine
  scans it (secret detection, risky-change checks, etc.). Any findings are
  recorded on the run and echoed to the agent as advisory feedback on stderr.
  This path needs no API key.

### Install

1. Make sure Trace is installed and the daemon can run (`trace daemon start`).
2. Copy this folder somewhere stable, e.g.:
   ```bash
   mkdir -p ~/.trace/integrations/claude
   cp trace-hook.sh ~/.trace/integrations/claude/
   chmod +x ~/.trace/integrations/claude/trace-hook.sh
   ```
3. Merge `settings.snippet.json` into your Claude Code `settings.json`
   (now includes both the `PreToolUse` and `PostToolUse` entries).
4. Install `jq` if you don't already have it. The script works without it,
   but falls back to a cruder text extraction for the JSON payload that can
   mis-parse edits containing quotes inside quotes — `jq` makes the
   PostToolUse path reliable on real-world code, not just simple diffs.

### Behaviour

- If the daemon is **not** running, the hook is a no-op — it never blocks Claude.
- If Claude wasn't launched via `trace run` (no `TRACE_RUN_ID` in its
  environment), the `PostToolUse` path is a no-op too — there's no run to
  attach the review to. `PreToolUse` command guarding still works either way.
- Every edit goes through the deterministic policy engine and any findings
  land on the dashboard (and are echoed to the agent as advisory feedback).
  The engine is fast, local, and needs no API key. Nothing that slips through
  is irreversible — the rollback path is how you undo it.
- `PreToolUse` command guarding blocks a `block`-classified command outright
  (exit 2) before it runs.

The hook only ever talks to `127.0.0.1` — nothing leaves the machine.
