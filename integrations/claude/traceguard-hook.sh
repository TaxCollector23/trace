#!/usr/bin/env bash
# TraceGuard adapter for Claude Code hooks.
#
# Reads the hook JSON payload from stdin, asks the local TraceGuard daemon to
# classify any Bash command, records an event, and blocks the command (exit 2)
# when the guard decision is "block".
#
# Configure it as a PreToolUse hook (see README.md). If the daemon is not
# running, the hook is a no-op and never blocks Claude.
set -euo pipefail

STATE="$HOME/.traceguard/daemon.json"
[ -f "$STATE" ] || exit 0  # daemon not running -> observe-only, do not block

PORT=$(sed -n 's/.*"port"[: ]*\([0-9]*\).*/\1/p' "$STATE" | head -1)
[ -n "${PORT:-}" ] || exit 0
BASE="http://127.0.0.1:${PORT}"

PAYLOAD=$(cat)

# Extract the command for Bash tool calls. (Best-effort without jq.)
COMMAND=$(printf '%s' "$PAYLOAD" | sed -n 's/.*"command"[: ]*"\(.*\)".*/\1/p' | head -1)
[ -n "${COMMAND:-}" ] || exit 0

RESP=$(curl -fsS -m 3 -X POST "${BASE}/api/check-command" \
  -H 'content-type: application/json' \
  -d "{\"command\": \"$(printf '%s' "$COMMAND" | sed 's/"/\\"/g')\"}" 2>/dev/null || echo '{}')

DECISION=$(printf '%s' "$RESP" | sed -n 's/.*"decision"[: ]*"\([a-z_]*\)".*/\1/p')

case "$DECISION" in
  block)
    echo "TraceGuard blocked this command." >&2
    exit 2  # non-zero blocks the tool call in Claude Code
    ;;
  *)
    exit 0
    ;;
esac
