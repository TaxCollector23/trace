#!/usr/bin/env bash
# Trace guard for Cursor's `beforeShellExecution` hook.
#
# Cursor sends the shell command it is about to run as JSON on stdin. This
# script classifies it with the Trace daemon's deterministic command guard and
# returns Cursor's permission decision on stdout:
#
#   {"permission":"allow"}                 -> let it run
#   {"permission":"deny","user_message":…} -> block it, tell the user + agent
#
# It FAILS OPEN: if the daemon is not running or anything goes wrong, the
# command is allowed, so Trace can never wedge Cursor. This is the enforced
# counterpart to Cursor's MCP tools (which are advisory).
set -euo pipefail

allow() { printf '{"permission":"allow"}'; exit 0; }

STATE="$HOME/.trace/daemon.json"
[ -f "$STATE" ] || allow

# Prefer a port handed down by `trc run`; else read the current daemon's.
PORT="${TRACE_DAEMON_PORT:-}"
if [ -z "$PORT" ]; then
  PORT=$(sed -n 's/.*"port"[: ]*\([0-9]*\).*/\1/p' "$STATE" | head -1)
fi
[ -n "${PORT:-}" ] || allow
BASE="http://127.0.0.1:${PORT}"

PAYLOAD=$(cat)

if command -v jq >/dev/null 2>&1; then
  COMMAND=$(printf '%s' "$PAYLOAD" | jq -r '.command // empty')
else
  COMMAND=$(printf '%s' "$PAYLOAD" | sed -n 's/.*"command"[: ]*"\(.*\)".*/\1/p' | head -1)
fi
[ -n "${COMMAND:-}" ] || allow

RESP=$(curl -fsS -m 3 -X POST "${BASE}/api/check-command" \
  -H 'content-type: application/json' \
  -d "{\"command\": \"$(printf '%s' "$COMMAND" | sed 's/\\/\\\\/g; s/"/\\"/g')\"}" 2>/dev/null || echo '{}')

DECISION=$(printf '%s' "$RESP" | sed -n 's/.*"decision"[: ]*"\([a-z_]*\)".*/\1/p')
REASON=$(printf '%s' "$RESP" | sed -n 's/.*"reason"[: ]*"\([^"]*\)".*/\1/p')

if [ "$DECISION" = "block" ]; then
  MSG="Trace blocked this command: ${REASON:-matched a high-risk rule}"
  if command -v jq >/dev/null 2>&1; then
    jq -cn --arg m "$MSG" '{permission:"deny",user_message:$m,agent_message:$m}'
  else
    printf '{"permission":"deny","user_message":"%s","agent_message":"%s"}' "$MSG" "$MSG"
  fi
  exit 0
fi

allow
