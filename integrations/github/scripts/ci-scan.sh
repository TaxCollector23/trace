#!/usr/bin/env bash
# TraceGuard CI scan.
#
# Runs configured checks, scans the diff against the base ref for risky changes
# and secret-like content, and writes a SANITIZED summary to
# traceguard-summary.json. It never writes raw file contents or secret values.
set -euo pipefail

CHECKS=""
FAIL_ON_RISKY="false"
while [ $# -gt 0 ]; do
  case "$1" in
    --checks) CHECKS="$2"; shift 2 ;;
    --fail-on-risky) FAIL_ON_RISKY="$2"; shift 2 ;;
    *) shift ;;
  esac
done

BASE_REF="${GITHUB_BASE_REF:-}"
if [ -n "$BASE_REF" ]; then
  git fetch --no-tags --depth=50 origin "$BASE_REF" || true
  RANGE="origin/${BASE_REF}...HEAD"
else
  RANGE="HEAD~1...HEAD"
fi

CHANGED_FILES=$(git diff --name-only "$RANGE" 2>/dev/null || echo "")
DIFF=$(git diff "$RANGE" 2>/dev/null || echo "")
NUM_FILES=$(printf '%s\n' "$CHANGED_FILES" | grep -c . || true)

# --- Risky-change heuristics (names only; never contents) ---
RISKY=0
echo "$CHANGED_FILES" | grep -Eiq '(^|/)(\.env($|\.)|id_rsa$|.*\.pem$|secrets\.json$)' && RISKY=$((RISKY+1)) || true

# --- Secret-like patterns in the diff (count only; values are NOT recorded) ---
SECRET_HITS=$(printf '%s' "$DIFF" | grep -Eoc \
  'sk-ant-[A-Za-z0-9_-]{20,}|sk-[A-Za-z0-9]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----' \
  || true)

# --- Configured checks ---
CHECK_STATUS="skipped"
if [ -n "$CHECKS" ]; then
  CHECK_STATUS="passed"
  IFS=',' read -ra CMDS <<< "$CHECKS"
  for cmd in "${CMDS[@]}"; do
    trimmed="$(echo "$cmd" | sed 's/^ *//;s/ *$//')"
    [ -z "$trimmed" ] && continue
    echo "TraceGuard check: $trimmed"
    if ! bash -c "$trimmed"; then
      CHECK_STATUS="failed"
    fi
  done
fi

# --- Sanitized summary (no file contents, no secret values) ---
cat > traceguard-summary.json <<EOF
{
  "schema": "traceguard.summary/v1",
  "commit": "${GITHUB_SHA:-unknown}",
  "files_changed": ${NUM_FILES:-0},
  "risky_file_warnings": ${RISKY},
  "secret_like_findings": ${SECRET_HITS:-0},
  "checks_status": "${CHECK_STATUS}"
}
EOF

echo "TraceGuard summary:"; cat traceguard-summary.json

if [ "$FAIL_ON_RISKY" = "true" ] && { [ "${SECRET_HITS:-0}" -gt 0 ] || [ "$RISKY" -gt 0 ]; }; then
  echo "::error::TraceGuard detected risky changes or secret-like content."
  exit 1
fi
