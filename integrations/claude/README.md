# TraceGuard — Claude Code integration

## Wrapper (recommended, works everywhere)

```bash
trg run claude
trg run "claude fix the login bug"
```

## Hooks adapter (optional, finer-grained)

`traceguard-hook.sh` is a `PreToolUse` hook that asks the local daemon to
classify Bash commands and blocks them (exit code 2) when the guard decision is
`block`.

### Install

1. Make sure TraceGuard is installed and the daemon can run (`trg daemon start`).
2. Copy this folder somewhere stable, e.g.:
   ```bash
   mkdir -p ~/.traceguard/integrations/claude
   cp traceguard-hook.sh ~/.traceguard/integrations/claude/
   chmod +x ~/.traceguard/integrations/claude/traceguard-hook.sh
   ```
3. Merge `settings.snippet.json` into your Claude Code `settings.json`.

### Behaviour

- If the daemon is **not** running, the hook is a no-op — it never blocks Claude.
- Maps the Claude session to a TraceGuard run when used together with
  `trg run claude`.
- Falls back to wrapper-based monitoring when hooks are unavailable.

The hook only sends the command string to `127.0.0.1`; nothing leaves the
machine.
