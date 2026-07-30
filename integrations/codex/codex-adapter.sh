#!/usr/bin/env bash
# Thin shim: route Codex CLI invocations through TraceGuard.
#
# Place this on your PATH ahead of the real `codex`, or alias it. It forwards
# all arguments to `trg run "codex ..."` so the session is recorded.
set -euo pipefail

if ! command -v trg >/dev/null 2>&1; then
  echo "traceguard: trg not found on PATH; running codex directly." >&2
  exec codex "$@"
fi

# Quote args back into a single command string for the wrapper.
CMD="codex"
for arg in "$@"; do
  CMD="$CMD $(printf '%q' "$arg")"
done

exec trg run "$CMD"
