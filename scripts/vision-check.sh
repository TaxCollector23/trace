#!/usr/bin/env bash
set -euo pipefail

# Run the north-star checks against a disposable Trace home. This script is
# intentionally local-only: provider credentials are never required and no
# network call is made. Provider-backed proxy checks are reported as an
# explicit gap unless the caller supplies an opt-in smoke command.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/trace-vision.XXXXXX")"
TRACE_HOME="$TEMP_ROOT/trace-home"
PROJECT="$TEMP_ROOT/project"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BIN="$TARGET_DIR/debug/trc"

cleanup() {
  if [ -x "$BIN" ]; then
    TRACE_HOME="$TRACE_HOME" TRACE_DB="$TRACE_HOME/trace.db" "$BIN" daemon stop >/dev/null 2>&1 || true
  fi
  rm -rf "$TEMP_ROOT"
}
trap cleanup EXIT

export TRACE_HOME
export TRACE_DB="$TRACE_HOME/trace.db"
export XDG_CONFIG_HOME="$TEMP_ROOT/config"
mkdir -p "$TRACE_HOME" "$PROJECT" "$XDG_CONFIG_HOME"

pass() { printf 'PASS  %s\n' "$1"; }
gap() { printf 'GAP   %s\n' "$1"; }

printf '%s\n' 'Trace vision check (disposable local state)'
printf '  repo: %s\n' "$ROOT"
printf '  data: %s\n' "$TRACE_HOME"

if [ "${TRACE_VISION_SKIP_TESTS:-0}" != "1" ]; then
  cargo test --workspace --exclude trace-desktop
fi
if [ "${TRACE_VISION_SKIP_BUILD:-0}" != "1" ]; then
  cargo build -p trace-cli
fi

if [ "${TRACE_VISION_SKIP_WEB:-0}" != "1" ] && command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/apps/web" && npm ci --ignore-scripts && npm run build)
  pass 'dashboard surface: the embedded web app builds'
else
  gap 'dashboard surface: npm/web build was skipped'
fi

if [ ! -x "$BIN" ]; then
  echo "trc binary not found at $BIN" >&2
  exit 1
fi

"$BIN" --version >/dev/null
pass 'install surface: CLI binary starts and reports a version'

mkdir -p "$PROJECT"
git -C "$PROJECT" init -q
git -C "$PROJECT" config user.email trace-vision@example.invalid
git -C "$PROJECT" config user.name trace-vision
printf '# vision fixture\n' > "$PROJECT/README.md"
git -C "$PROJECT" add README.md
git -C "$PROJECT" commit -qm initial

(cd "$PROJECT" && "$BIN" init >/dev/null)
pass 'zero-config local state: project initialization succeeds in a fresh home'

(cd "$PROJECT" && "$BIN" run 'printf vision-check' >/dev/null)
if (cd "$PROJECT" && "$BIN" runs | grep -q completed); then
  pass 'live run: a real wrapped command is recorded'
else
  echo 'wrapped command did not produce a completed run' >&2
  exit 1
fi

if (cd "$PROJECT" && "$BIN" self-check | grep -q 'All fixtures and red-team threats passed.'); then
  pass 'deterministic safety benchmark: self-check passes'
else
  echo 'self-check did not report a passing benchmark' >&2
  exit 1
fi

if "$BIN" proxy --help >/dev/null 2>&1; then
  pass 'proxy surface: trc proxy is discoverable'
else
  gap 'proxy surface: CLI wiring is not present yet'
fi

if [ "${TRACE_VISION_PROVIDER_SMOKE:-0}" = "1" ]; then
  if [ -z "${OPENAI_API_KEY:-}" ] \
    && [ -z "${ANTHROPIC_API_KEY:-}" ] \
    && [ -z "${GOOGLE_API_KEY:-}" ] \
    && [ -z "${GEMINI_API_KEY:-}" ]; then
    gap 'provider smoke: requested but no provider key is available'
  else
    gap 'provider smoke: opt-in command is not automated by default; run a provider-specific check and attach its output'
  fi
else
  gap 'provider smoke: not run; Trace never requires or collects provider credentials'
fi

gap 'historical import: agent-specific discovery/import is not proven by this clean baseline'
gap 'live dashboard stream: server-sent events/websocket and ticking usage are not proven'
gap 'budgets and alerts: configuration, threshold persistence, and native notifications are not proven'
gap 'exports and multi-machine bundles: round-trip checks are not present'

printf '%s\n' 'Vision check completed. GAP lines are tracked in docs/vision-gaps.md.'
