#!/usr/bin/env bash
# Trace adapter for Claude Code hooks.
#
# Handles two hook events, dispatched on `hook_event_name` in the JSON
# payload Claude Code sends on stdin:
#
#   PreToolUse (Bash)        -> classify the command with the rule-based
#                                guard; block (exit 2) on a "block" decision.
#   PostToolUse (Edit/Write/  -> send the edit to Trace's live review path
#     MultiEdit)                (deterministic policy engine, and — only
#                                when Model Prompting Mode is on — the 3-LLM
#                                judge panel). If the panel says to stop,
#                                exit 2 so Claude Code shows the reason back
#                                to the agent as feedback on that tool call.
#
# If the daemon is not running, every path here is a no-op and never blocks
# Claude — this hook only ever adds friction when Trace is actively watching.
set -euo pipefail

STATE="$HOME/.trace/daemon.json"
[ -f "$STATE" ] || exit 0

# Prefer the port passed down by `trace run` (TRACE_DAEMON_PORT) so this
# still works correctly if a second daemon instance is ever running; fall
# back to whatever's recorded as the current daemon.
PORT="${TRACE_DAEMON_PORT:-}"
if [ -z "$PORT" ]; then
  PORT=$(sed -n 's/.*"port"[: ]*\([0-9]*\).*/\1/p' "$STATE" | head -1)
fi
[ -n "${PORT:-}" ] || exit 0
BASE="http://127.0.0.1:${PORT}"

PAYLOAD=$(cat)

HAVE_JQ=0
command -v jq >/dev/null 2>&1 && HAVE_JQ=1

json_get() {
  # json_get <top-level-or-tool_input key>
  local key="$1"
  if [ "$HAVE_JQ" = "1" ]; then
    printf '%s' "$PAYLOAD" | jq -r --arg k "$key" '(.[$k] // .tool_input[$k] // empty)'
  else
    # Best-effort fallback: first "<key>": "<value>" match. Breaks on
    # embedded escaped quotes in the value — install jq for reliable
    # PostToolUse handling on edits with complex content.
    printf '%s' "$PAYLOAD" | sed -n "s/.*\"${key}\"[: ]*\"\\([^\"]*\\)\".*/\\1/p" | head -1
  fi
}

EVENT=$(json_get hook_event_name)

case "$EVENT" in
  PreToolUse)
    COMMAND=$(json_get command)
    [ -n "${COMMAND:-}" ] || exit 0

    RESP=$(curl -fsS -m 3 -X POST "${BASE}/api/check-command" \
      -H 'content-type: application/json' \
      -d "{\"command\": \"$(printf '%s' "$COMMAND" | sed 's/"/\\"/g')\"}" 2>/dev/null || echo '{}')

    DECISION=$(printf '%s' "$RESP" | sed -n 's/.*"decision"[: ]*"\([a-z_]*\)".*/\1/p')
    if [ "$DECISION" = "block" ]; then
      echo "Trace blocked this command." >&2
      exit 2
    fi
    exit 0
    ;;

  PostToolUse)
    # Only file-editing tools are worth a live review pass.
    TOOL=$(json_get tool_name)
    case "$TOOL" in
      Edit|Write|MultiEdit|NotebookEdit) ;;
      *) exit 0 ;;
    esac

    RUN_ID="${TRACE_RUN_ID:-}"
    [ -n "$RUN_ID" ] || exit 0  # not launched via `trace run` — nothing to attach this to

    FILE_PATH=$(json_get file_path)
    # Coarse content signal for the policy engine / judge — whichever of
    # these the tool used. Truncated; this is a review signal, not a full
    # patch.
    DIFF=$(json_get new_string)
    [ -n "$DIFF" ] || DIFF=$(json_get content)
    DIFF=$(printf '%s' "$DIFF" | head -c 4000)

    if [ "$HAVE_JQ" = "1" ]; then
      BODY=$(jq -n --arg tn "$TOOL" --arg fp "$FILE_PATH" --arg d "$DIFF" \
        '{tool_name: $tn, file_path: $fp, diff_summary: $d}')
    else
      ESC_DIFF=$(printf '%s' "$DIFF" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n' ' ')
      BODY="{\"tool_name\": \"$TOOL\", \"file_path\": \"$FILE_PATH\", \"diff_summary\": \"$ESC_DIFF\"}"
    fi

    RESP=$(curl -fsS -m 30 -X POST "${BASE}/api/runs/${RUN_ID}/hook-check" \
      -H 'content-type: application/json' \
      -d "$BODY" 2>/dev/null || echo '{}')

    if [ "$HAVE_JQ" = "1" ]; then
      BLOCK=$(printf '%s' "$RESP" | jq -r '.block // false')
      FEEDBACK=$(printf '%s' "$RESP" | jq -r '.agent_feedback // .message // empty')
      CONSENSUS=$(printf '%s' "$RESP" | jq -r '.consensus // empty')
    else
      # Best-effort without jq — breaks on messages with escaped quotes.
      # Install jq for reliable multi-line feedback with per-model reasoning.
      BLOCK=$(printf '%s' "$RESP" | sed -n 's/.*"block"[: ]*\(true\|false\).*/\1/p')
      FEEDBACK=$(printf '%s' "$RESP" | sed -n 's/.*"message"[: ]*"\(.*\)","policy.*/\1/p')
      CONSENSUS=$(printf '%s' "$RESP" | sed -n 's/.*"consensus"[: ]*"\([a-z_]*\)".*/\1/p')
    fi

    if [ "$BLOCK" = "true" ]; then
      # Blocking path: exit 2 surfaces the message back to Claude Code as
      # feedback on the tool call. `agent_feedback` includes each reviewer's
      # specific reasoning, so the model sees exactly what to fix — the
      # whole point of Model Prompting Mode.
      echo "${FEEDBACK:-Trace's review panel flagged this edit. Please re-examine it before continuing.}" >&2
      exit 2
    fi

    # Non-blocking advisory: policy engine or judge returned warn. Echo to
    # stderr and exit 0 so the agent sees the feedback (and can self-correct
    # on its next tool call) without being interrupted.
    if [ -n "${FEEDBACK:-}" ] && [ "${CONSENSUS:-allow}" != "allow" ]; then
      echo "$FEEDBACK" >&2
    fi
    exit 0
    ;;

  *)
    exit 0
    ;;
esac
