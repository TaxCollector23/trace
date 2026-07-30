# Trace: merged architecture

This documents how Trace and Ratify were combined, and why, so the decisions
aren't just implicit in the diff.

## The core call: one runtime, not two

Trace was a local-first Rust daemon (SQLite, a rule-based guard, rollback)
watching AI coding agents on your machine. Ratify was a Next.js service
triggered by GitHub webhooks, reviewing pull requests with a deterministic
policy engine plus a single LLM reasoner.

Rather than keep both stacks and have them call each other, **Ratify's
review brain was ported into Trace's Rust core** (`trace-core`):

- `policy.rs` — the deterministic checks (secret scanning, missing tests on
  sensitive paths, swallowed errors, direct DB access in handlers, etc.),
  ported line-for-line in behavior from `policy-checks.ts`.
- `judge.rs` — a new 3-LLM consensus panel, replacing Ratify's single
  OpenRouter-fallback-chain reasoner. Extends the same idea Ratify's
  `confidence-blending.ts` used (corroboration between sources raises
  confidence) to three independent models instead of "policy engine +
  one LLM."
- `prompt_quality.rs` — new; heuristic prompt-quality scoring that feeds the
  prompting-analytics dashboard (Phase 2).

Why this direction and not the reverse (keep Ratify's TS service, have Trace
call it)? Two reasons that matter for what you actually asked for:

1. **Live intervention needs to be local and fast.** Phase 3 — Trace
   noticing an agent doing something wrong *while it's still working* and
   prompting it to fix things — has to run in the same process that's
   already watching the agent's file changes and commands. A round trip to
   a hosted Next.js service on every file write is not workable latency,
   and depending on a hosted service being up is not something a local
   guardrail tool should require.
2. **One engine, two consumers.** The same `policy.rs` + `judge.rs` will
   power both the live local daemon *and* the CI/PR-review path (a
   `trace review-pr` subcommand + GitHub Action, replacing Ratify's
   webhook handler) — see "Deferred" below. Duplicate implementations in
   Rust and TypeScript would drift.

Ratify's Next.js app, GitHub App plumbing, and Postgres/Drizzle schema are
retired as a separate deployment; their logic is absorbed, not thrown away.

## Data model

New SQLite tables in `trace-core/src/db.rs`, all keyed off the existing
`runs` table so a policy finding or judge verdict is always attached to the
run (and therefore the project, timeline, and rollback point) it came from:

- `policy_findings` — one row per deterministic-engine hit.
- `judge_verdicts` / `judge_votes` — one verdict row per analysis pass, with
  each of the panel's individual votes attached. `action_taken` records
  whether Model Prompting Mode actually sent something back to the agent
  (`agent_prompted`) or only logged it (`flagged_only`), which is exactly
  the toggle you described.
- `prompt_events` — one row per user prompt to the agent, scored by
  `prompt_quality.rs`. This is what Phase 2's dashboard reads for prompting
  coaching.

## The judge: how the 3-LLM panel actually works

`crates/trace-core/src/judge.rs`. Three provider slots (Anthropic, OpenAI,
Google by default, each independently swappable) are called concurrently.
Each returns a decision (`allow` / `warn` / `require_approval` / `block`), a
confidence, and a one-line reason. The panel's consensus is majority vote,
confidence-blended with a corroboration boost when models agree — and,
importantly, **the judge can only escalate the deterministic guard's
decision, never downgrade it**. A rule that blocks `rm -rf` for being
catastrophic doesn't get talked down to "allow" by an LLM panel; the panel
can only add caution on top of what's already deterministic.

Two key-supply modes, both implemented, matching what you asked for:

- **`OwnKeys`** — the user's own provider keys, read from `~/.trace/global.toml`
  or environment variables (`TRACE_ANTHROPIC_API_KEY` etc.), used only to
  call each provider directly from the local machine. Nothing passes
  through Trace's servers. Good for: users who don't want their code/prompts
  touching a third party, or who already have provider credits they'd
  rather spend directly.
- **`BackendProxy`** — a single Trace-hosted endpoint that holds the keys,
  fans the request out to three models server-side, and meters usage
  against the user's Trace account. Good for: users who don't want to
  manage three API keys, or want judge usage billed rather than metered
  against their own provider accounts. *(The proxy server itself isn't part
  of this repo — it's the "Trace backend" referenced in `judge.rs`; wiring
  the actual hosted endpoint is a deployment task, not a code one.)*

Nothing in the binary contains a hardcoded key for either mode — that was a
real risk in the original ask and is called out explicitly in `judge.rs`'s
module docs.

## Cross-platform (Windows, Linux)

The premise going in was "only works on macOS." After actually auditing it,
that turned out to be narrower than it sounded: `trace-core`/`trace-cli`/
`trace-daemon` already had real `#[cfg(unix)]`/`#[cfg(windows)]` branches for
most things (agent detection, clipboard, shelling out, signal handling), and
the CLI/daemon binary already cross-compiled for macOS/Linux/Windows in CI.
The actual gaps, and what was done about each:

1. **The desktop GUI shell only shipped for macOS.** `tauri.conf.json` had a
   single PNG icon and a `macOS`-only bundle section; CI only built and
   published a `.dmg`. Fixed: generated the full icon set (`.ico` for
   Windows, `.icns` for macOS, the PNG sizes Linux wants) via `tauri icon`,
   added real `windows` (NSIS) and `linux` (deb + AppImage) bundle configs,
   and added `desktop-windows`/`desktop-linux` jobs to `release.yml` plus a
   3-OS matrix in `ci.yml`'s smoke-build job.
2. **`daemon_ctl.rs`'s `detach()` was a silent no-op on Windows.** The whole
   point of that function is making the background daemon survive the CLI
   process exiting — on Unix it calls `setsid()`; on Windows it did nothing,
   so the daemon stayed attached to the parent console and could die with it
   (or catch a Ctrl+C meant for the CLI). Fixed with
   `CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS` via `CommandExt::creation_flags`.
3. **`trace update` (self-update) would fail on Windows.** Unix lets you
   rename a new binary directly over the one currently executing; Windows
   holds an exclusive lock on the running exe's file and refuses. Fixed by
   renaming the running exe aside first (`.trace-update.old`) before moving
   the new one into place — a well-known pattern for this exact problem.

Everything else audited clean: file watching (`notify`'s `recommended_watcher`
already picks the right native backend per OS), the installers
(`install.sh` already branches Darwin/Linux correctly, `install.ps1` already
existed and looks correct), asset embedding (`rust-embed` keys are
OS-independent), and process spawning elsewhere in the CLI.

Not independently verified: I don't have a Windows or Linux desktop
environment in this sandbox to actually launch the Tauri app bundle and
click through it. The `cargo check`/`tsc`/`vite build` level checks that
could run in this sandbox all passed; an actual run of `trace-desktop` on a
real Windows machine and a real Linux desktop is the next honest checkpoint,
not something I can claim from here.


- **Doctrine mining** (`ratify/.../doctrine/miner.ts`) — mines review rules
## AI layer: confident-dissent escalation

`judge.rs::aggregate` originally used pure majority vote (with a
cautious-tiebreak) to reach consensus. That has a real weakness: two
lukewarm "allow" votes could outvote a single reviewer who is 0.95-confident
they spotted something serious (a committed secret, disabled auth), which
defeats the actual point of asking three independent models instead of one.

Added: a single vote at ≥0.85 confidence toward *more* caution than the
majority now escalates the consensus to that vote's decision, regardless of
count. Deliberately one-directional — a confident *"allow"* can never talk a
majority *"block"* down; only escalation is possible, never relaxation. This
mirrors the existing rule that the judge panel as a whole can only escalate
the deterministic guard's decision, extended one level down to how the panel
resolves its own internal disagreement.

Verified with five test cases (now permanent tests in `judge.rs`), including
the critical safety property as its own named test:
`confident_dissent_can_never_de_escalate`.

## Hardening pass

- **CORS**: was `CorsLayer::permissive()` — wildcard, meaning any webpage the
  user had open in a normal browser tab could script requests against this
  daemon (read config, trigger rollbacks, kick off doctrine mining using the
  user's GitHub token, spam paid judge calls). Replaced with an explicit
  origin allow-list (just the Vite dev server; the packaged app is
  same-origin and needs no CORS grant at all).
- **CSP**: `tauri.conf.json`'s `csp` setting doesn't actually apply here —
  the desktop shell points its webview at the daemon via
  `WebviewUrl::External`, not Tauri's own asset protocol, so Tauri never
  injects a policy into it. Added a real `Content-Security-Policy` (plus
  `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`) as a
  response header from the daemon itself instead — which has the added
  benefit of covering the other legitimate way to view the dashboard, a
  plain browser pointed at `http://127.0.0.1:<port>` via `trace dashboard`.
- **Judge cooldown memory growth**: the per-run cooldown map (added to cap
  API spend from rapid save-loops) had no eviction — one entry per run id,
  forever, on a daemon meant to run for days/weeks. Now evicts stale entries
  opportunistically on every check.

Both the CORS and middleware changes were verified against the real axum 0.7
/ tower-http 0.5 API surface in an isolated crate (this sandbox's Rust
toolchain is too old for the full workspace, see earlier notes) — actually
compiled and run, not written from memory and assumed correct.

## Deferred (still not done, and why)

- **CI/PR-review path**: done. `trace review-diff` (new CLI subcommand,
  `crates/trace-cli/src/commands/review_diff.rs`) runs the same
  `policy.rs`/`judge.rs` engine over a git diff range — no daemon, no
  `trace init`, works from a bare CI checkout. The GitHub Action now
  installs `trace` and calls it, with a grep-based fallback if install
  fails for some reason. Tested end-to-end in this sandbox (a real git repo
  with a planted secret correctly fails the scan; a clean diff passes).
- **Doctrine mining**: done, simplified from the original plan. Rather than
  porting Ratify's full GitHub App (installation tokens, webhooks — real
  infrastructure a hosted service needs, not a local CLI tool), mining
  reuses Trace's existing lightweight token resolution
  (`github.rs::resolve_token` — env var / `gh` CLI / config file) since it's
  a user-triggered, read-only, local operation. `doctrine.rs` mines rules
  from a project's merged-PR review comments using whichever judge provider
  is configured, stores them per-project, and feeds them into both
  `analyze_run` and `hook_check`'s judge prompts. Dashboard: a Doctrine
  panel on the Judge Panel page (project picker, "Mine doctrine" button,
  rule table).
- **Live agent-intervention delivery**: full two-way delivery (agent sees
  and acts on the feedback) is still Claude Code-only, because it's the
  only adapter with a hook surface Trace can inject into
  (`trace-hook.sh`'s `PostToolUse` handler). For every other wrapped agent
  (Cursor, Aider, Codex, OpenCode, Gemini), the file watcher
  (`watcher.rs::review_live`) now runs the *same* policy+judge review live,
  on every debounced file save, using the same `/api/runs/:id/hook-check`
  endpoint — but since Trace is watching from outside the tool-call loop
  for these, a "block" verdict becomes a loud terminal alert to the human
  at the keyboard plus a dashboard flag, not something the agent itself
  sees. That's an honest, real difference between "has a hook API" and
  "gets watched from outside," not something more engineering effort here
  would close — it needs each tool to expose a feedback surface first.
- **Multi-model judge**: no longer capped at three hardcoded labs.
  `judge.rs::call_provider_raw` has a generic OpenAI-compatible catch-all
  (`base_url` on `ProviderSlot`) covering DeepSeek, Mistral, xAI, Groq,
  Together, OpenRouter, a local Ollama server, or anything else that speaks
  that wire format — no new code needed per provider. The Judge Panel UI
  lets you add/remove slots beyond the default three.

## Bugs found and fixed along the way

Worth naming explicitly rather than burying in a diff — these were real
correctness problems, not style nits:

- `analyze_run` was feeding the policy engine a `"+12 -3"` stat string
  instead of real diff content. Its regex checks (secrets, TODOs, swallowed
  catches) had nothing to match against — they were silently inert for
  every local run. Fixed with `git.rs::patches_by_file`/`split_diff_by_file`
  (unit-tested with plain `rustc`, since the sandbox's Rust toolchain is too
  old for a full `cargo test`).
- `ChangeType::as_str()` produces `"created"/"deleted"`, but `policy.rs`'s
  checks (matching GitHub's own vocabulary) test for `"added"/"removed"`.
  `check_missing_tests`, `check_removed_tests`, and `check_migration_added`
  could never fire correctly wherever a stored `ChangeType` fed into a
  `FileDiff.status`. Fixed by adding `ChangeType::as_diff_status()` for that
  vocabulary specifically, rather than changing `as_str()` and risking a
  silent behavior change everywhere it's already used for display/storage.
- The judge's per-provider HTTP timeout (30s) exceeded the Claude Code
  hook's own timeout (25s) — a slow provider could time out the hook
  script *after* the daemon had already committed to waiting on it,
  producing a silent non-block instead of a clear failure. Tightened to a
  20s/30s split with headroom.
