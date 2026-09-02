# Trace — Recovery Audit (implementation as source of truth)

Date: 2026-09-02 · Branch: `hardening-phases` → `main` · Binary: `target/release/trc` (reports `Trace 1.3`)

This is a **read-only, evidence-based** audit of what Trace actually does today, per the
recovery-prompt Part 1 mandate ("audit before changing anything; implementation is the source
of truth, not the README"). Six independent agents inspected the source, ran the real binary,
and drove each subsystem with live payloads in disposable sandboxes. No project source, git
history, or `~/.trace` data was modified during the audit.

Verdict legend: **PASS** (works e2e and honest) · **PARTIAL** (real but with material gaps) ·
**FAIL** (does not deliver its stated guarantee).

---

## Scorecard

| Pillar / surface | Verdict | One-line reason |
|---|---|---|
| SEE — daemon / persistence / dashboard data | PARTIAL | Run status silently coerced to `running`; no test/prod data separation |
| SEE — CLI + first-run UX | FAIL | `--help` hides `run`/`dashboard`/`doctor`; `init` calls a clean repo dirty |
| STOP — command guard | PARTIAL | Strong on literal patterns; trivially evaded by `$IFS`/quoting/hex/env-indirection |
| STOP — secret handling | FAIL (on the stated guarantee) | Raw secrets persisted in `commands.command`, `stdout.log`, `diff.patch.gz` |
| STOP — policy engine + benchmark | PARTIAL→FAIL (methodology) | `self-check` reports 100% by construction; no real per-rule precision/recall |
| UNDO — checkpoints + rollback | FAIL | Common dirty-checkpoint case aborts and restores nothing; never an exact restore |
| GitHub (ratify / review-diff / action / webhook) | PARTIAL | `ratify`/`review-diff` are genuinely e2e; webhook is an unsigned stub |
| Distribution integrity | PARTIAL | SHA-256 verified, but fail-open when the sidecar is absent; no signature/provenance |
| CI | FAIL (for the stated bar) | No e2e smoke test; fmt short-circuits tests; installer/action code only syntax-checked |
| Agent integrations | Mixed | OpenCode PASS, Cursor PASS (command guard), Claude PARTIAL, Codex/Windsurf/VS Code observe-only |

**The single fact that governs enforcement:** every hook/plugin calls `POST /api/check-command`
→ `guard::classify` → `allow / warn / require_approval / block`. **Only `block` is enforced
anywhere.** `require_approval` and `warn` are returned to the agent boundary as *allow*
(proven: `git reset --hard HEAD~1` classifies `require_approval` yet Cursor/Claude/OpenCode all
let it run). "Require approval" is advisory-only at the agent surface today.

---

## SEE

### Daemon / persistence / dashboard — PARTIAL

- **P0 — run status is silently coerced to `running`.** `finish()` maps any unknown/interrupted
  status through a catch-all `_ => RunStatus::Running` (`crates/trace-core/src/models.rs:31`,
  `crates/trace-daemon/src/api.rs:237`). A Ctrl-C'd or crashed run is recorded as still
  *running* forever; there is no `ABORTED`/`INTERRUPTED` state and no zombie reconciliation on
  daemon restart. This is the root of the "junk / stuck runs" the owner sees.
- **P0 — no test/prod data separation.** The DB is always `~/.trace/trace.db`; there is no
  `TRACE_HOME`/`TRACE_DB` override and no `trc reset`/purge. Development, tests, and real usage
  all pollute one store.
- **P1 — secrets duplicated and, on disk, un-redacted** (see STOP/secrets below).
- Dashboard renders no fabricated panels (good), but faithfully renders the polluted/mis-stated
  data above. VERSION reports `1.3` vs the workspace's `1.3.x`.

### CLI + first-run — FAIL

- **P0-1 — the product surface is hidden from `--help`.** `run`, `dashboard`, `doctor`, `scan`,
  `check`, `runs`, `show`, `rollback`, `update`, `self-check`, `review-diff`, `ratify`, `github`
  are all `#[command(hide=true)]` (`crates/trace-cli/src/main.rs`). Real `trc --help` lists only
  `init`, `integrations`, `daemon`, `help` — a first-run user cannot discover `trc run` or
  `trc dashboard`, the core value.
- **P0-2 — `trc init` warns your clean repo is dirty using its own files.** init writes
  `.trace/` then reports "Working tree has uncommitted changes" (`init.rs:115`); `.trace/.gitignore`
  only ignores `runs/`, so `.trace/` itself dirties the parent repo.
- **P1 — legitimate CI-gate failures look like crashes.** `check`/`self-check` signal via
  `anyhow::bail!`, routing an intended non-zero exit through the generic "Re-run with TRACE_DEBUG=1"
  error path. `ratify` does this right (`std::process::exit(1)`).
- **P1 — raw HTTP internals leak as errors** (`error: GET /api/runs/<id>` instead of "run not found").

## STOP (the deterministic safety pillar)

### Command guard — PARTIAL

Covered (real `trc check` blocks): `rm -rf /` across flag/`sudo`/`--no-preserve-root` forms,
`curl … | sh` incl. `sudo`, base64-to-shell, `&& sh x` download-then-exec, reverse shells,
`nc -e`, dd/mkfs/`>/dev/sd*`, fork bomb, chained-destructive-after-benign, infra teardown
(`aws s3 rm --recursive`, terraform/kubectl) as `require_approval`, git force-push as `warn`.

**Evaded — all pass CLEAN (real output):** `$IFS` separators (`rm${IFS}-rf${IFS}/`), nested/empty
quoting (`r""m -rf /`), env-indirection (`X=rm; $X -rf /`), hex/unicode (`printf "\x72\x6d…"|sh`),
bare `printf|sh`/`echo|sh`, chmod-then-exec, and interpreter inline code (`python3 -c "…os.system…"`).
Structural cause: substring/token matching on normalized-but-not-unescaped text — no shell
tokenizer, no quote-stripping, no variable expansion. A motivated agent bypasses it trivially.

### Secret handling — FAIL (on the "Trace never stores raw secrets" claim)

Redaction (`secrets.rs::redact`) is real and correct **for the derived `secrets` table only**.
But the raw secret is persisted verbatim in three places:
1. **`commands.command`** — full raw command line (`run.rs:118`, `db.rs:363`). A secret on the
   command line (`export API_KEY=…`, `curl -H "Authorization: Bearer …"`) lands in the DB in clear.
2. **`stdout.log` / `stderr.log`** — `tee_stream` writes each line verbatim (`run.rs:439`).
3. **`diff.patch.gz`** — the full unified diff is gzipped to disk (`run.rs:210`).

Redaction happens at the *findings* boundary, not the *storage* boundary. Format coverage is
decent (28 patterns) but misses Azure/GCP service-account JSON, Twilio auth token, generic
high-entropy secrets, and Basic-auth headers.

### Policy engine + benchmark — PARTIAL→FAIL (methodology)

The engine runs the real `run_policy_checks` (not mocked); the fixtures are legitimate. But the
benchmark is **per-rule unit testing dressed as precision/recall**:
- A positive fixture passes if the one targeted rule fired; **extra/unrelated findings are
  explicitly allowed and never counted as false positives** (`eval.rs:609`). Proven live: a
  Stripe-key fixture also fired `missing-tests-for-payments-paths` and still `[PASS]` at "100%
  precision".
- False positives accrue only on `expected_rule: None` fixtures, so precision can never drop from
  a positive case. No per-rule confusion matrix; the corpus contains none of the guard evasions
  above — which is why it reports 0 misses while real bypasses exist.
- `trc self-check` headline (`45/45 · precision 100% · recall 100%`, `73/73 threats`) is a
  designed outcome, not a measured one.

### Versioning — three out-of-sync schemes

`Cargo.toml` = `1.3.3` · hardcoded `VERSION="1.3"` drives `--version`/doctor
(`crates/trace-core/src/lib.rs:9`) · rule pack = `2025.08.4` (`default_pack.toml:10`). No single
command shows all of {Trace version, pack version, rule count}; `--version`/`doctor` expose
neither the pack version nor any rule count.

## UNDO — checkpoints + rollback — FAIL

Verified with live before/after diffs in disposable repos.

- **Capture** (`git.rs:107`): dirty tree → `git stash create` **without `-u`**, so untracked and
  ignored files the agent created are never captured. Clean tree → HEAD hash.
- **Restore** (`git.rs:291`): a parent-count heuristic routes stashes to `git stash apply` and
  commits to `git reset --hard`.
- **CRITICAL** — the common real case (tree dirty at checkpoint, agent edits those same files):
  `git stash apply` **aborts on conflict and restores nothing** while the daemon reports an error.
  Undo does nothing exactly when it is most needed.
- **HIGH** — even when it succeeds, `stash apply` is additive, not an exact restore: post-checkpoint
  deletions aren't reverted and untracked files remain. No backup of the pre-rollback tree, so a
  failed rollback is itself unrecoverable.
- **MEDIUM** — merge-commit checkpoints are mis-routed to `stash apply`; unreferenced `stash create`
  objects can be `git gc`'d before rollback.

To make "undo" trustworthy the checkpoint must capture the complete worktree and restore must force
the tree to exactly match the snapshot after backing up current state.

## GitHub / Distribution / CI

- **GitHub — PARTIAL.** `trc ratify <PR>` and `trc review-diff --range` are genuinely e2e, keyless,
  daemon-free, deterministic, and CLI/daemon-consistent (verified on live PRs: PR#1 → `block (3 high)`,
  PR#2 → `pass`). Gaps: `trc github *` needs `trc init`; the webhook app (`handler.js`) is a
  placeholder with **no `X-Hub-Signature-256` verification**; three real bugs — `ci-scan.sh`
  high-severity grep is always 0 (space-mismatch vs pretty JSON), `handler.js` reads stale summary
  fields, and **`scripts/install.sh` uses `REPO="TaxCollector23/trc"`** while every other channel
  uses `…/trace` (broken download path).
- **Distribution — PARTIAL.** SHA-256 verification is implemented in npm/install.sh/install.ps1
  (verify-before-write, atomic rename) and fails closed on mismatch, but is **fail-open when the
  sidecar is absent** (unless `TRACE_REQUIRE_CHECKSUM`), defaults to `latest`, and has **no
  signature/attestation/provenance** (the `.sha256` shares the binary's origin). Homebrew (pinned +
  checksummed) is the one fully-solid channel.
- **CI — FAIL (for the stated bar).** Real fmt/clippy/test gating exists, but there is **no e2e
  smoke test** that runs the shipped binary, fmt short-circuits tests (same sequential job), and
  installer/action code is only syntax-checked — which is exactly why the two runtime bugs above
  slipped through.

## Agent integrations

| Agent | Command enforce | Edit/file | Verdict |
|---|---|---|---|
| OpenCode | ✓ plugin `tool.execute.before` throws on `block` (pre-exec, proven) | via `trc run`/MCP | **PASS** |
| Cursor | ✓ `beforeShellExecution` deny + `agent_message` (proven) | no file hook | **PASS** (command guard) |
| Claude Code | ✓ PreToolUse Bash → exit 2 (proven) | edit review **advisory-only, never blocks** (`hook_check` hardcodes `block:false`), only with `TRACE_RUN_ID` | **PARTIAL** |
| Codex | only top-level `codex …` classified via `trc run`; sub-commands invisible | FS/git observe via wrapper | **observe-only** |
| Windsurf | ✗ no hook | MCP read-only | **observe-only** |
| VS Code | ✗ no shell hook | time-window save observer, UI-only warnings | **observe-only / not installable via `trc integrations`** |

OpenCode is correctly the reference: interception is inside the agent's own pre-execution hook,
with a real thrown exception the agent surfaces, and correct fail-open when the daemon is down.

---

## Priority-ordered fix list (foundation first)

1. **Run-state correctness + data separation** (SEE P0s): add `ABORTED`/`INTERRUPTED`, stop
   coercing to `running`, reconcile zombies on restart; add `TRACE_HOME`/`TRACE_DB` and
   `trc reset --local-data`. *Unblocks trustworthy dashboards and the v4 intelligence layer.*
2. **Redact at the storage boundary** (STOP secrets): scrub `commands.command`, `stdout/stderr.log`,
   and `diff.patch.gz` through the scanner, not just the findings table.
3. **Rollback fidelity** (UNDO): capture the full worktree (`-u`) and restore exactly, backing up
   current state first; never report a no-op abort as anything but a failure.
4. **First-run UX** (CLI): unhide `run`/`dashboard`/`doctor`/…; capture git state before writing
   `.trace/`; stop routing gate failures through the crash path.
5. **Honest benchmark methodology**: count extra findings on positive fixtures as FPs; real
   per-rule precision/recall/F1; expose pack version + rule count in `--version`/`doctor`.
6. **CI smoke test**: run the shipped binary through init/run/check/rollback; fix the `install.sh`
   repo slug and `ci-scan.sh` severity grep; stop fmt short-circuiting tests.
7. **Guard hardening** (bounded): tokenizer-aware normalization for `$IFS`/quoting/env-indirection,
   and enforce `require_approval` at the agent boundary (not just `block`).
8. **Distribution**: consider fail-closed-by-default + provenance/attestation; pin by default.

Everything in the v4 "intelligent control room" prompt depends on items 1–2 being true first:
an intelligence layer built on telemetry that mis-reports run outcomes and stores raw secrets
would violate that prompt's own rules ("never fabricate", "surface integrity issues",
"redaction before any export/LLM/report").
