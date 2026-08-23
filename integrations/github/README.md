# Trace — GitHub integration

Two ready-to-use surfaces plus an App skeleton.

## GitHub Action (`action.yml`)

Run Trace's **deterministic** policy engine in CI and upload a **sanitized**
summary artifact. No API key required — the engine is pure pattern matching.

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
```

The action installs `trace` (unless `install: "false"`) and runs
`trc review-diff` — the same deterministic policy engine the local daemon
uses, not a separate implementation — over the PR's diff. Falls back to a
crude grep-based heuristic if `trace` couldn't be installed, so the action
still does something useful rather than failing outright.

The scan (`scripts/ci-scan.sh`) writes two files:
- `trace-summary.json` — counts only (files changed, finding counts, check
  status). Safe to glance at in a PR comment bot later.
- `trace-review.json` — the full structured result from `trc review-diff`
  (finding titles/descriptions). Still never includes raw secret values — the
  policy engine redacts those at the source (see `trace-core/src/policy.rs`).

Neither file ever contains raw project file contents or your local SQLite
database.

## Running it yourself, outside the Action

```bash
trc review-diff --range origin/main...HEAD --fail-on-risky --json trace-review.json
```

Works from any git checkout — no `trc init`, no daemon required. Useful
for testing what CI will see before you push, or wiring into a different CI
system entirely.

To review a **GitHub pull request** by number instead of a local diff, use:

```bash
trc ratify <pr-number> --fail-on-risky
```

which fetches the PR's files from GitHub and runs the same engine.

## GitHub App (`app/`)

- `app.manifest.json` — create the App from this manifest, then set your webhook
  URL and generate a private key.
- `handler.js` — dependency-free event-handler skeleton for `pull_request`,
  `push`, and posting a check run.

## Guarantees

The integration uploads **only sanitized summaries**. It never uploads raw
project files, raw secrets, or your local SQLite database.

