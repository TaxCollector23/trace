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
  edit to Trace's live review path: the deterministic policy engine always
  runs; the 3-LLM judge panel additionally runs when **Model Prompting
  Mode** is on in the dashboard's Judge Panel. If the panel lands on
  `require_approval` or `block`, the hook exits 2 with the panel's reasoning
  on stderr — Claude Code shows this to the agent as feedback on that edit,
  which is what prompts it to stop and reconsider before continuing.

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
- **Model Prompting Mode off (default):** every edit still goes through the
  deterministic policy engine and any findings land on the dashboard, but
  nothing is sent back to the agent and nothing blocks — the existing
  rollback path is how you undo something that slipped through. This keeps
  editing fast.
- **Model Prompting Mode on:** the 3-LLM judge panel runs on every
  file-editing tool call. This adds real latency per edit (the panel is
  three model round-trips) in exchange for Trace being able to actually
  interrupt the agent when it's about to compound a mistake. Turn it on for
  higher-stakes sessions, off for fast iteration — see the Judge Panel page
  in the dashboard.

The hook only talks to `127.0.0.1`; nothing leaves the machine except the
judge panel's own calls to whichever LLM providers you've configured (see
Judge Panel → Judge settings for the two key-supply modes).
