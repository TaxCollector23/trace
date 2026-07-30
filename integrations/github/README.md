# TraceGuard — GitHub integration

Two ready-to-use surfaces plus an App skeleton.

## GitHub Action (`action.yml`)

Run TraceGuard checks in CI and upload a **sanitized** summary artifact.

```yaml
# .github/workflows/traceguard.yml
name: traceguard
on: [pull_request]
jobs:
  traceguard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 50 }
      - uses: TaxCollector23/TraceGuard/integrations/github@main
        with:
          checks: "npm run build, npm test"
          fail-on-risky: "true"
```

The scan (`scripts/ci-scan.sh`) records **counts only** — number of files
changed, risky file-name warnings, secret-like findings, and check status. It
never writes file contents or secret values.

## GitHub App (`app/`)

- `app.manifest.json` — create the App from this manifest, then set your webhook
  URL and generate a private key.
- `handler.js` — dependency-free event-handler skeleton for `pull_request`,
  `push`, and posting a check run.

## Guarantees

The integration uploads **only sanitized summaries**. It never uploads raw
project files, raw secrets, or your local SQLite database.
