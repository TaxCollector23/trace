# Trace — GitHub integration

Two ready-to-use surfaces plus an App skeleton.

## GitHub Action (`action.yml`)

Run Trace's policy engine — and, optionally, the 3-LLM judge panel — in CI
and upload a **sanitized** summary artifact.

```yaml
# .github/workflows/trace.yml
name: trace
on: [pull_request]
jobs:
  trace:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 50 }
      - uses: TaxCollector23/trace/integrations/github@main
        with:
          checks: "npm run build, npm test"
          fail-on-risky: "true"
          enable-judge: "true"
        env:
          # Any of the three (or more — see the Judge Panel in the
          # dashboard for adding providers). Only set the ones you use.
          TRACE_ANTHROPIC_API_KEY: ${{ secrets.TRACE_ANTHROPIC_API_KEY }}
          TRACE_OPENAI_API_KEY: ${{ secrets.TRACE_OPENAI_API_KEY }}
          TRACE_GOOGLE_API_KEY: ${{ secrets.TRACE_GOOGLE_API_KEY }}
```

The action installs `trace` (unless `install: "false"`) and runs
`trace review-diff` — the same deterministic policy engine and judge panel
the local daemon uses, not a separate implementation — over the PR's diff.
Falls back to a crude grep-based heuristic if `trace` couldn't be installed,
so the action still does something useful rather than failing outright.

The scan (`scripts/ci-scan.sh`) writes two files:
- `trace-summary.json` — counts only (files changed, finding counts, judge
  consensus, check status). Safe to glance at in a PR comment bot later.
- `trace-review.json` — the full structured result from `trace review-diff`
  (finding titles/descriptions, judge votes and reasoning). Still never
  includes raw secret values — the policy engine redacts those at the
  source (see `trace-core/src/policy.rs`).

Neither file ever contains raw project file contents or your local SQLite
database.

## Running it yourself, outside the Action

```bash
trace review-diff --range origin/main...HEAD --judge --fail-on-risky --json trace-review.json
```

Works from any git checkout — no `trace init`, no daemon required. Useful
for testing what CI will see before you push, or wiring into a different CI
system entirely.

## GitHub App (`app/`)

- `app.manifest.json` — create the App from this manifest, then set your webhook
  URL and generate a private key.
- `handler.js` — dependency-free event-handler skeleton for `pull_request`,
  `push`, and posting a check run.

## Guarantees

The integration uploads **only sanitized summaries**. It never uploads raw
project files, raw secrets, or your local SQLite database.

