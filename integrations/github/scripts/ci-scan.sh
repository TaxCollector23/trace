#!/usr/bin/env bash
# Trace CI scan.
#
# When the `trace` binary is available, delegates to `trace review-diff` —
# the same deterministic policy engine the local daemon uses (no API key
# required) — and folds its findings into trace-summary.json. Falls back to a
# crude grep-based heuristic when `trace` isn't installed, so this action
# still does *something* useful without requiring an install step, and
# never hard-fails just because the binary is missing.
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
NUM_FILES=$(printf '%s\n' "$CHANGED_FILES" | grep -c . || true)

POLICY_FINDINGS=0
HIGH_SEVERITY=0
REVIEW_FAILED="false"

if command -v trace >/dev/null 2>&1; then
  echo "Trace: using 'trace review-diff' (deterministic policy engine)"
  FAIL_FLAG=""
  [ "$FAIL_ON_RISKY" = "true" ] && FAIL_FLAG="--fail-on-risky"

  set +e
  # shellcheck disable=SC2086
  trace review-diff --range "$RANGE" $FAIL_FLAG --json trace-review.json
  REVIEW_EXIT=$?
  set -e

  if [ -f trace-review.json ]; then
    POLICY_FINDINGS=$(grep -o '"rule_key"' trace-review.json | wc -l | tr -d ' ')
    HIGH_SEVERITY=$(grep -o '"severity":"high"' trace-review.json | wc -l | tr -d ' ')
  fi
  [ "$REVIEW_EXIT" != "0" ] && REVIEW_FAILED="true"
else
  echo "Trace: 'trace' binary not found on PATH — falling back to basic heuristics."
  echo "For the full policy engine, install it first, e.g.:"
  echo "  curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh"

  DIFF=$(git diff "$RANGE" 2>/dev/null || echo "")
  RISKY=0
  echo "$CHANGED_FILES" | grep -Eiq '(^|/)(\.env($|\.)|id_rsa$|.*\.pem$|secrets\.json$)' && RISKY=$((RISKY+1)) || true
  SECRET_HITS=$(printf '%s' "$DIFF" | grep -Eoc \
    'sk-ant-[A-Za-z0-9_-]{20,}|sk-[A-Za-z0-9]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----' \
    || true)
  POLICY_FINDINGS=$((RISKY + ${SECRET_HITS:-0}))
  HIGH_SEVERITY=${SECRET_HITS:-0}
  [ "$FAIL_ON_RISKY" = "true" ] && [ "$POLICY_FINDINGS" -gt 0 ] && REVIEW_FAILED="true"
fi

# --- Configured checks ---
CHECK_STATUS="skipped"
if [ -n "$CHECKS" ]; then
  CHECK_STATUS="passed"
  IFS=',' read -ra CMDS <<< "$CHECKS"
  for cmd in "${CMDS[@]}"; do
    trimmed="$(echo "$cmd" | sed 's/^ *//;s/ *$//')"
    [ -z "$trimmed" ] && continue
    echo "Trace check: $trimmed"
    if ! bash -c "$trimmed"; then
      CHECK_STATUS="failed"
    fi
  done
fi

# --- Sanitized summary (no file contents, no secret values) ---
cat > trace-summary.json <<EOF
{
  "schema": "trace.summary/v3",
  "commit": "${GITHUB_SHA:-unknown}",
  "files_changed": ${NUM_FILES:-0},
  "policy_findings": ${POLICY_FINDINGS:-0},
  "high_severity_findings": ${HIGH_SEVERITY:-0},
  "checks_status": "${CHECK_STATUS}"
}
EOF

echo "Trace summary:"; cat trace-summary.json

if [ "$CHECK_STATUS" = "failed" ]; then
  echo "::error::Trace: one or more configured checks failed."
  exit 1
fi
if [ "$REVIEW_FAILED" = "true" ]; then
  echo "::error::Trace detected risky changes (see trace-review.json / trace-summary.json)."
  exit 1
fi
