# Vision gaps

This is the honest baseline for `scripts/vision-check.sh`. A green command or
unit test proves a local source path; it does not prove an agent's private log
format, a provider invoice, or a hosted deployment.

## Current gaps

- Historical agent discovery/import is not guaranteed on a clean install. Each
  importer needs a real local observation, a golden fixture, and an incremental
  restart test before this becomes a pass.
- Live dashboard updates, budgets, threshold alerts, and native notifications
  are not implemented as one verified end-to-end flow.
- Export formats and multi-machine bundle deduplication need round-trip tests.
- Provider smoke tests are opt-in. Trace must never require a provider key in
  CI or store one. A proxy response can provide real token usage; its computed
  amount remains API-equivalent pricing, not an invoice.

## Verification rule

Only move an item out of this file after running the corresponding command or
fixture flow on a disposable Trace home and recording the evidence in the
change that claims it. Missing, opaque, or provider-specific fields remain
`unavailable`; they are never replaced with zero.
