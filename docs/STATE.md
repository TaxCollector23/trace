# Trace implementation state

Updated: 2026-10-12

This is an evidence map for the checked-out implementation. A passing local
build proves source consistency; it does not prove a GitHub Actions run, a
provider billing reconciliation, or a live deployment.

## Recovery and release disposition

| Area | Current state | Evidence boundary |
| --- | --- | --- |
| Run status recovery | fixed | `RunStatus::Unknown`, `Aborted`, and `Interrupted` are explicit; daemon startup reconciles orphaned running rows. |
| Local/test data separation | fixed | `TRACE_HOME`, `TRACE_DB`, and `trc reset --local-data` are implemented and tested. |
| Secret persistence | guarded at storage boundaries | Run, command, event, diff, test, and secret writes redact; the legacy plaintext capture path still needs a disposable-home end-to-end audit before claiming universal proof. |
| Command-guard evasions | measured, not claimed perfect | The red-team corpus includes shell evasions and the real guard result is exposed by `trc self-check`; misses or downgrades fail the gate. |
| Benchmark methodology | fixed in source | Policy fixtures expose per-rule TP/FP/FN, precision, recall, and F1; red-team engines expose caught, missed, downgraded, and false-positive counts. |
| Installer checksums | fail closed by default | Shell, PowerShell, landing, and npm installers require a 64-character SHA-256 sidecar; `TRACE_ALLOW_UNSIGNED=1` is an explicit escape hatch. Live release execution remains a separate proof. |
| Release provenance | configured | Tagged CLI artifacts are staged with checksums and GitHub build provenance attestation; the tagged workflow has not been run from this checkout. |
| CI smoke paths | configured | Rust fmt/clippy/tests, CLI end-to-end, usage/fixture smoke, installer container smoke, web/landing builds, and syntax checks are in `.github/workflows/ci.yml`. |

## Canonical usage boundary

Trace stores accounting metadata only: provider/agent, model, session and
message identity, token classes, timestamps, subagent linkage, source
location, and cost provenance. Transcript content is not part of the usage
event model.

| Surface | Current behavior |
| --- | --- |
| Claude Code | Incremental `~/.claude/projects/**/*.jsonl` importer; sidechains are linked conservatively; incomplete final lines are retried; malformed lines are counted. |
| Codex, OpenCode, Cursor, Aider | Read-only local metadata importers under `crates/trace-core/src/importers`; `trc import <agent|all>` writes the same `usage_events` table and persists source cursors/diagnostics. |
| Storage | Idempotent `(agent, session_id, message_id)` upsert; additive migrations preserve existing stores. |
| Cost | A versioned `crates/trace-core/data/pricing.json` catalog computes only when provider/model/tokens/time/rates are sufficient. Provider-reported and computed values are stored separately with reconciliation status and pricing version. |
| API/CLI | `GET /api/usage` and `trc usage` are the first aggregate consumer, with model/session/day/agent/project grouping. `trc verify --day` reports missing/unknown pricing rather than converting it to zero. |
| Privacy | Importers never persist prompt or response content; `raw_ref` is a path plus stable locator. |

## Experimental boundary

The supported product is the local CLI, daemon, SQLite store, embedded browser
dashboard, local landing/docs, integrations, and GitHub Action. Retired
surfaces are preserved under `experimental/` for reference only:

- `experimental/desktop-shell/` is the former Tauri wrapper.
- `experimental/cloud-api/` and `experimental/render.yaml` are the former
  hosted service and deployment blueprint.
- `experimental/firebase/` is old static-hosting scaffolding.
- `experimental/github-app/` is a webhook sketch; the maintained path is the
  GitHub Action plus `trc review-diff`/`trc ratify`.

They are not Cargo workspace members, CI jobs, release artifacts, landing
routes, or product claims. The daemon has no cloud-sync client.

## Verification boundary

Local verification for this integration pass includes `cargo fmt --all`,
workspace checks/tests, importer fixtures, web build/smoke checks, landing
build, installer syntax, and `git diff --check`. It does not establish a live
Vercel deployment or a completed GitHub Actions workflow until those targets
are checked after the final commit.
