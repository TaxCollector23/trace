#!/usr/bin/env bash
# Thin shim: route Codex CLI invocations through Trace.
#
# Place this on your PATH ahead of the real `codex`, or alias it. It forwards
# all arguments to `trace run "codex ..."` so the session is recorded.
set -euo pipefail

if ! command -v trace >/dev/null 2>&1; then
  echo "trace: trace not found on PATH; running codex directly." >&2
  exec codex "$@"
fi

# Quote args back into a single command string for the wrapper.
CMD="codex"
for arg in "$@"; do
  CMD="$CMD $(printf '%q' "$arg")"
done

exec trace run "$CMD"
