# Trace — Architecture

This is the deep reference for how Trace is built: every crate, every module,
the data model, the request paths, and the reasoning behind the important
decisions. It is meant to be read start-to-finish once and skimmed afterwards.

If you only remember one sentence: **Trace is one Rust engine (`trace-core`)
with several thin front-ends (a daemon, a CLI, an embedded web dashboard, a
marketing site), and every detection it performs is deterministic — no LLM, no
API key, ever.**

---

## Table of contents

1. [What Trace is](#1-what-trace-is)
2. [Design principles](#2-design-principles)
3. [The big picture](#3-the-big-picture)
4. [Workspace layout](#4-workspace-layout)
5. [`trace-core` — the engine](#5-trace-core--the-engine)
6. [The detection engines in detail](#6-the-detection-engines-in-detail)
7. [The rule pack (data-driven coverage)](#7-the-rule-pack-data-driven-coverage)
8. [The data model (SQLite)](#8-the-data-model-sqlite)
9. [`trace-daemon` — the local server](#9-trace-daemon--the-local-server)
10. [`trace-cli` — the `trace` binary](#10-trace-cli--the-trace-binary)
11. [How a monitored run works, step by step](#11-how-a-monitored-run-works-step-by-step)
12. [Agent adapters](#12-agent-adapters)
13. [Agent integrations (hooks)](#13-agent-integrations-hooks)
14. [Ratify — deterministic PR review](#14-ratify--deterministic-pr-review)
15. [Benchmarks (self-check & red-team)](#15-benchmarks-self-check--red-team)
16. [The web dashboard](#16-the-web-dashboard)
17. [The landing site](#17-the-landing-site)
18. [The desktop app](#18-the-desktop-app)
19. [`trace-cloud-api` — optional cloud sync](#19-trace-cloud-api--optional-cloud-sync)
20. [The no-API-key guarantee](#20-the-no-api-key-guarantee)
21. [History: the TraceGuard → Trace → deterministic path](#21-history-the-traceguard--trace--deterministic-path)
22. [Testing](#22-testing)
23. [Data & privacy boundaries](#23-data--privacy-boundaries)
24. [Complete surface reference](#24-complete-surface-reference)

---

## 1. What Trace is

Trace is a **local black-box recorder and safety layer for AI coding agents**
(Claude Code, Codex, Cursor, Aider, Gemini, OpenCode, Windsurf, …). You launch
an agent under Trace and it:

- **checkpoints your git state** before anything changes, so you can always
  roll back;
- **watches every file the agent touches** and records the authoritative diff;
- **classifies every command** the agent runs against a rule-based guard
  (`allow` / `warn` / `require_approval` / `block`);
- **scans everything for secrets** and redacts them;
- **runs a deterministic policy engine** over the diff (missing tests on
  sensitive paths, swallowed errors, hardcoded localhost, secrets in a diff,
  …);
- **tracks token/cost usage** for the session;
- **surfaces all of it in a local dashboard** — timeline, patch review, command
  risk, cost, and one-click rollback.

It also **ratifies GitHub pull requests**: run the same deterministic policy
engine over a PR's changed files and get a `pass` / `review` / `block` verdict —
from the dashboard, the CLI, or the API.

Everything runs on your machine. Nothing is sent anywhere unless you explicitly
opt into cloud sync.

## 2. Design principles

1. **Deterministic, not probabilistic.** Every check is pattern matching over
   real text — a regex against added diff lines, a normalized-command lookup, a
   secret-shaped-string detector. The same input always produces the same
   output, it runs in microseconds, and it requires **no API key**. This is a
   guardrail, and a guardrail that sometimes phones a slow, paid, non-
   deterministic third party is not a guardrail you can trust in the tool-call
   path.
2. **One engine, many consumers.** The guard, the secret scanner, the policy
   engine, the ratify verdict — each has exactly one implementation in
   `trace-core`, shared verbatim by the daemon, the CLI, CI, and the dashboard.
   Two implementations drift; one cannot.
3. **Local-first.** The daemon binds `127.0.0.1` only. The database is a single
   SQLite file under `~/.trace`. The dashboard is embedded into the daemon
   binary. Nothing needs the network to work.
4. **Honest about limits.** A GUI tool launched under `trace run` is observed
   via file changes and the final git diff, but its internal actions can't be
   guarded. The docs and code say so rather than pretending otherwise.
5. **Coverage as data.** Detection rules that are pure data (injection phrases,
   supplemental command rules, extra secret patterns) live in a versioned
   `.toml` "rule pack" that can be updated without recompiling the binary — the
   "virus-definitions" model.

## 3. The big picture

```
                       ┌──────────────────────────────────────────┐
                       │                trace-core                 │
                       │  guard · secrets · policy · ratify ·      │
                       │  redteam · eval · rules_pack · db · git · │
                       │  github · cost · scan · adapters · models │
                       └──────────────────────────────────────────┘
                          ▲            ▲             ▲          ▲
             depends on   │            │             │          │
              ┌───────────┘            │             │          └────────────┐
              │                        │             │                       │
     ┌────────────────┐      ┌──────────────────┐    │              ┌─────────────────┐
     │   trace-cli    │      │   trace-daemon   │    │              │  apps/desktop   │
     │  (the `trace`  │─────▶│  Axum server on  │    │              │  (Tauri shell)  │
     │    binary)     │ HTTP │  127.0.0.1:8757  │    │              └─────────────────┘
     └────────────────┘      │                  │    │
              │              │  embeds ─────────┼────┘
              │              │  apps/web/dist   │  (rust_embed, compile time)
              │              └──────────────────┘
              │                        ▲
    launches / wraps                   │ /api/*
              ▼                        │
     ┌────────────────┐      ┌──────────────────┐      ┌──────────────────┐
     │  coding agent  │      │    apps/web      │      │   apps/landing   │
     │ (claude, etc.) │      │  (dashboard SPA) │      │ (marketing site) │
     └────────────────┘      └──────────────────┘      └──────────────────┘

   trace-cloud-api  —  a *separate*, optional hosted service (does not depend
                       on trace-core); the daemon can push sanitized run
                       summaries to it if the user opts in.
```

Data flows one direction on the way in and the reverse on the way out:

```
agent action ─▶ CLI wrapper / agent hook ─▶ daemon endpoint ─▶ trace-core engine
                                                                      │
                              dashboard ◀─ /api/* ◀─ SQLite ◀─────────┘
```

## 4. Workspace layout

A Cargo workspace (`Cargo.toml` at the root) plus a set of front-end apps.

```
trace/
├── Cargo.toml                 # workspace: 4 Rust crates + the Tauri crate
├── crates/
│   ├── trace-core/            # the engine — all shared logic, no I/O server
│   ├── trace-daemon/          # local Axum HTTP server + embedded dashboard
│   ├── trace-cli/             # the `trace` binary (clap subcommands)
│   └── trace-cloud-api/       # optional hosted sync service (standalone)
├── apps/
│   ├── web/                   # the dashboard (React SPA, embedded in daemon)
│   ├── landing/               # public marketing site (React)
│   ├── desktop/               # Tauri desktop shell (src-tauri = trace-desktop)
│   └── docs/                  # docs site source (mdx under /docs)
├── integrations/
│   ├── claude/                # PostToolUse/PreToolUse hook for Claude Code
│   ├── github/                # GitHub Action + App skeleton + ci-scan.sh
│   └── …                      # cursor, windsurf, etc.
├── docs/                      # the .mdx documentation pages
├── homebrew-trace/            # Homebrew tap formula
├── scripts/                   # install.sh / install.ps1 and release tooling
└── firebase/ , render.yaml    # hosting config for the landing + cloud API
```

The workspace shares one version (`workspace.package.version`) and a release
profile tuned for small binaries (`opt-level = "z"`, LTO, `strip`, `panic =
"abort"`).

## 5. `trace-core` — the engine

`crates/trace-core/src/`. No servers, no UI, no long-running I/O. Everything the
daemon and CLI both need lives here so the two can never diverge. `lib.rs`
declares the modules and re-exports the public API. The product version string
(`VERSION`, `version_string()`) is defined here as the single source of truth.

| Module | Responsibility |
| --- | --- |
| `guard.rs` | Classify a shell command → `Decision` (`Allow`/`Warn`/`RequireApproval`/`Block`) with a human reason. Understands evasions (pipe-to-shell, download-then-exec, base64-to-shell, raw block-device writes, `find … -delete`, fork bombs, DB drops). |
| `secrets.rs` | Detect secret-shaped strings (provider API keys, tokens, private keys, JWTs, DB URLs, bearer tokens) and produce a redacted representation. Also `is_env_like_filename` for protected-file logic. |
| `policy.rs` | The deterministic diff-review engine: `run_policy_checks(&[FileDiff]) -> Vec<PolicyFinding>`. Secret-in-diff, removed test files, swallowed catch blocks, hardcoded localhost, dependency changes, TODO/debug markers, etc. Each finding has a typed `Severity`. |
| `prompt_quality.rs` | Heuristic scoring of a prompt (`analyze_prompt`) and `prompt_risks` — the prompt-risk detector used by the red-team benchmark (embedded dangerous commands, injection phrases, leaked secrets). |
| `ratify.rs` | `summarize(&[PolicyFinding]) -> RatifySummary` — turn policy findings into a `pass`/`review`/`block` verdict with per-severity counts. Shared by the daemon endpoint and the `trace ratify` CLI. |
| `redteam.rs` | The adversarial benchmark corpus + `run_redteam_eval()`. Feeds dangerous commands, planted secrets, and unsafe prompts through the *real* engines and scores recall / false positives. |
| `eval.rs` | The policy-engine's labeled-fixture benchmark: `run_policy_eval()` → precision/recall over hand-written fixtures. |
| `rules_pack.rs` + `default_pack.toml` | The versioned, data-driven rule pack (see §7). |
| `db.rs` | The SQLite store (`Store`). Schema creation, typed queries, row mappers. Keyed off the `runs` table. |
| `git.rs` | Git operations: checkpoints, `diff_range`, `patches_by_file`, `remote_url`, `is_git_repo`, change-type vocabulary translation. |
| `github.rs` | Read from the GitHub API: `parse_remote`, `resolve_token` (env / `gh` CLI / `~/.trace/github.json`), `list_commits`, `list_pulls`, `list_pr_files`, `get_file`, `status_for_path`. Read-only. |
| `cost.rs` | Token/cost accounting for a run. |
| `scan.rs` | Detect a project's stack (package manager, languages, frameworks, test framework, env files, deployment) for `trace scan`. |
| `adapter.rs` + `agents.rs` | `SessionContext` and per-agent metadata used by the CLI adapters. |
| `models.rs` | Plain serializable row/wire types (`Run`, `Project`, `FileChange`, `CommandRecord`, `SecretRecord`, `PolicyFindingRecord`, cost/checkpoint/test-result records, …). |
| `config.rs` | `ProjectConfig` (`<project>/.trace/config.toml`) — protected files, checks. |
| `diagnose.rs` | Backing logic for `trace doctor` health checks. |
| `paths.rs`, `ids.rs`, `time.rs` | `~/.trace` path helpers, id generation, RFC-3339 timestamps. |

The public re-exports (`lib.rs`) are the crate's contract: `classify`,
`Decision`, `GuardResult`, `run_policy_checks`, `FileDiff`, `PolicyFinding`,
`Severity`, `run_policy_eval`, `run_redteam_eval`, `ratify_summarize`,
`RatifyVerdict`, `Store`, `ProjectConfig`, and the model types.

## 6. The detection engines in detail

### The command guard (`guard.rs`)

`classify(command: &str) -> GuardResult { decision, reason }`. The command is
normalized (lowercased, whitespace collapsed) and matched against an
**ordered, descending-severity** list of built-in rules; the first match wins,
so the most restrictive applicable decision is what you get. Highlights:

- **Block** — `rm -rf /` and variants (incl. `--no-preserve-root`), pipe-to-
  shell (`curl … | sh`, and evasions like `curl … | sudo bash`, no-space
  `curl…|sh`), download-then-exec (`curl -o /tmp/x.sh && sh /tmp/x.sh`),
  base64-decode-to-shell, raw block-device writes (`dd of=/dev/sda`,
  `> /dev/sda`, `mkfs`), fork bombs, `find / -delete`, DB drops.
- **RequireApproval** — destructive-but-sometimes-intended: `rm -rf ~`, `git
  reset --hard`, `git clean -fd`, recursive `chown`, `terraform destroy`,
  `kubectl delete namespace`, `aws s3 rm --recursive`, mass row deletes.
- **Warn** — `chmod -R 777`, `git push --force`, reading a `.env`, `history -c`.
- **Allow** — everything else, including deliberate false-positive traps (a
  commit message that merely *mentions* `rm -rf`).

Supplemental command rules from the rule pack (§7) can only **escalate** a
built-in decision, never downgrade it.

### The secret scanner (`secrets.rs`)

`scan_text(&str) -> Vec<SecretFinding { secret_type, redacted_value }>`. Regex
detectors for a broad provider set (Anthropic, OpenAI, GitHub PATs, AWS access
keys and secret keys, Google, Groq, Stripe, Slack, SendGrid, GitLab, npm,
Twilio, …), plus SSH private keys, JWTs, DB connection URLs, and bearer tokens.
Every finding carries a **redacted** value — the raw secret never leaves the
detector.

### The policy engine (`policy.rs`)

`run_policy_checks(&[FileDiff]) -> Vec<PolicyFinding>`. It matches on **added
diff lines** (`+`-prefixed), so it reviews what a change *introduces*. Rules
include secret-in-diff (reusing the secret scanner), removed test files,
swallowed `catch` blocks, hardcoded `localhost`/`127.0.0.1` in production paths,
dependency-manifest changes, and leftover TODO/debug markers. Fixture paths are
ignored for secret rules to avoid flagging test data. This is the engine behind
live review, `trace review-diff`, and Ratify.

### The prompt-risk detector (`prompt_quality.rs`)

`prompt_risks(&str)` flags prompts that embed dangerous commands, contain
prompt-injection/jailbreak phrases, or leak secrets. `analyze_prompt` scores a
prompt's clarity heuristically. Used by the red-team benchmark's prompt engine.

## 7. The rule pack (data-driven coverage)

`rules_pack.rs` + `default_pack.toml`. Detection that is pure *data* —
injection phrases, supplemental command rules (`all_of` needle lists →
decision), and extra secret patterns (regex + how many chars to keep) — lives in
a calendar-versioned TOML pack rather than hard-coded Rust. The default pack is
**embedded in the binary** via `include_str!`. Pointing the `TRACE_RULES_PATH`
environment variable (legacy: `TRACEGUARD_RULES_PATH`) at a newer `.toml`
overrides it at runtime, so coverage can improve **without shipping a new
binary** — the "virus-definitions" model. A malformed override logs a warning
and falls back to the embedded default rather than leaving the tool ruleless.
The active pack is loaded once into a process-wide `Lazy`.

Complex rules that need real parsing (pipe-to-shell detection) stay in
`guard.rs`; the pack augments them.

## 8. The data model (SQLite)

`db.rs` opens a single SQLite database under `~/.trace`. Every table hangs off
the `runs` table so a command, file change, secret, cost record, or policy
finding is always attributable to a run (and therefore a project, a timeline,
and a rollback point).

Core tables: `projects`, `runs`, `events` (timeline), `file_changes`,
`commands`, `secrets`, `api_usage` (cost), `checkpoints`, `test_results`, and
`policy_findings`. Indices are created for each `run_id`/`project_id` foreign
key. The schema is created idempotently with `CREATE TABLE IF NOT EXISTS` on
open, so a fresh install and an upgrade both just work.

> Historical note: earlier versions also had `judge_verdicts`, `judge_votes`,
> `prompt_events`, and `doctrine_rules` tables for the LLM features that have
> since been removed (§21). New databases no longer create them; existing ones
> keep the now-unused tables harmlessly.

## 9. `trace-daemon` — the local server

`crates/trace-daemon/src/`. An [Axum](https://docs.rs/axum) HTTP server bound to
`127.0.0.1` (default port `8757`, auto-incrementing if taken).

- `server.rs` — builds the router, binds an available port, writes
  `~/.trace/daemon.json` (pid/port/started_at) on start and clears it on
  shutdown, applies a CORS layer and security headers.
- `state.rs` — `AppState`: an `Arc<Mutex<Store>>` plus port/started_at/db_path.
  (It used to also hold global judge config and a judge-call cooldown map; both
  are gone now that review is deterministic-only.)
- `api.rs` — all the route handlers.
- `assets.rs` — **embeds `apps/web/dist` into the binary at compile time** via
  `rust_embed`. Unknown non-`/api` paths fall back to `index.html` so the SPA's
  client-side routing works. This is why changing the dashboard means rebuilding
  `apps/web/dist` *and then* the daemon.
- `cloud_sync.rs` — optional push of sanitized summaries to `trace-cloud-api`.

The full route list is in §24. The two review endpoints —
`POST /runs/:id/analyze` and `POST /runs/:id/hook-check` — run the deterministic
policy engine only and return findings; the older `judge_verdict` /
`agent_instruction` fields are kept as `null` for wire-compatibility with agent
hooks that still read them.

## 10. `trace-cli` — the `trace` binary

`crates/trace-cli/src/`. `main.rs` defines the clap command tree; each command
lives in `commands/`. The binary is named `trace`.

| Command | What it does |
| --- | --- |
| `trace init` | Register the current project (writes `.trace/config.toml`). |
| `trace run "<cmd>"` | Run a command under full monitoring (see §11). |
| `trace check <file>` | Run a file (or `-` for stdin) through the guard + secret scanner; non-zero exit on `require_approval`/`block`. A CI gate for scripts. |
| `trace ratify <pr>` | Ratify a GitHub PR against the policy engine — no daemon, no key. `--fail-on-risky` exits non-zero on a `block` verdict. |
| `trace review-diff` | Review a git range with the policy engine; built for CI (`--range`, `--fail-on-risky`, `--json`). |
| `trace self-check` | Run the policy + red-team benchmarks and print the report. |
| `trace scan` | Detect and print the current project's stack. |
| `trace dashboard` | Open the local dashboard (starts the daemon if needed). |
| `trace doctor` | Health checks: toolchain, clipboard, daemon, agents, paths, policy self-check. |
| `trace runs` / `show` / `patch` / `risks` / `costs` / `checkpoints` / `replay` | Query recorded runs. |
| `trace rollback` | Roll back to the most recent git checkpoint. |
| `trace config show|set` | Project configuration. |
| `trace integrations [status|install <agent>]` | Manage agent hooks. |
| `trace github [status|commits|pulls|cat]` | Read from the project's GitHub repo. |
| `trace update` | Self-update to the latest GitHub release. |
| `trace daemon start|stop|status` | Manage the local daemon. |
| `trace __serve` | Hidden: run the server in the foreground (used by `daemon start`). |

## 11. How a monitored run works, step by step

`commands/run.rs`:

1. Load the current project (`project::load_current`).
2. **Guard the top-level command** with `guard::classify`. A `block` stops here;
   `require_approval` prompts (unless `-y`).
3. **Checkpoint git** so there's a rollback point.
4. Ensure the daemon is running (`daemon_ctl`), create a run record, and export
   `TRACE_RUN_ID` into the child's environment.
5. **Pick an adapter** (§12) based on the command, and launch the agent as a
   child process with its stdout/stderr **teed** (shown live *and* captured).
6. A **file watcher** (`watcher.rs`) debounces filesystem events and, for each
   changed file, POSTs to `/runs/:id/hook-check` — the deterministic policy
   engine scans it, findings are recorded, and any advisory is surfaced.
7. On exit, derive the **authoritative file changes from the final git diff**
   (not the noisy live events), scan the whole diff for secrets, and finish the
   run.

Everything lands in SQLite and shows up on the dashboard.

## 12. Agent adapters

`trace-cli/src/adapters/` — one small module per agent (`claude.rs`,
`cursor.rs`, `codex.rs`, `aider.rs`, `gemini.rs`, `opencode.rs`, `windsurf.rs`,
plus `terminal.rs` for a bare shell). An adapter normalizes an agent's output
and metadata so the run recorder is agent-agnostic. Adding a new agent is a new
adapter, not changes scattered through the run path.

## 13. Agent integrations (hooks)

`integrations/`. Agents that support hooks call Trace directly instead of being
wrapped:

- **Claude Code** (`integrations/claude/trace-hook.sh`): a `PreToolUse` hook
  classifies Bash commands with the guard (exit 2 blocks a `block` command
  before it runs), and a `PostToolUse` hook posts each edit to
  `/runs/:id/hook-check`. The deterministic engine scans it and the hook echoes
  any advisory back to the agent on stderr (exit 0). No API key involved.
- **GitHub** (`integrations/github/`): a composite Action (`action.yml`) that
  runs `ci-scan.sh` → `trace review-diff` over a PR's diff and uploads a
  **sanitized** summary (counts only; never raw files, secrets, or the local
  DB). A GitHub App skeleton (`app/`) is included for a webhook-driven setup.

`trace integrations install <agent>` writes the hook and patches the agent's
config file idempotently, with backups.

## 14. Ratify — deterministic PR review

Ratify runs the same policy engine that guards local edits over a **GitHub pull
request's changed files**, and reduces the findings to a single verdict:

- **block** — at least one high-severity finding (e.g. a committed secret);
- **review** — medium-severity findings only;
- **pass** — nothing flagged.

The verdict logic is `trace_core::ratify::summarize`, unit-tested and shared by
three surfaces so they can never disagree:

- **Dashboard** — the *Ratify* tab: pick a project, ratify an open PR or a PR by
  number, see findings + verdict.
- **API** — `GET /api/github/ratify?project_id=<id>&pr=<n>`.
- **CLI** — `trace ratify <pr> [--fail-on-risky]`, standalone from any checkout
  (resolves the origin remote and a read-only token itself).

`github::list_pr_files` fetches the PR's files; `run_policy_checks` produces the
findings; `summarize` produces the verdict. No LLM, no key.

## 15. Benchmarks (self-check & red-team)

Two labeled benchmarks run the **real** engines (no mocks) and are exposed
everywhere:

- **Policy eval** (`eval.rs`, `run_policy_eval`) — hand-written fixtures scored
  for precision/recall. Each rule is tested with a case that *should* fire and a
  deliberate near-miss that *shouldn't*.
- **Red-team** (`redteam.rs`, `run_redteam_eval`) — an adversarial corpus of
  dangerous commands (including evasions), planted secrets, and unsafe prompts,
  scored for recall and false positives across the three engines.

Both are surfaced by `trace self-check` (terminal), `/api/benchmarks` and
`/api/benchmarks/redteam` (computed fresh per request), and the dashboard's
*Benchmarks* page. `cargo run -p trace-core --example redteam_bench` prints the
detailed red-team report. They double as unit tests — a regression fails the
build.

## 16. The web dashboard

`apps/web/` — a React + Vite SPA, hash-routed (so deep links work when served as
static files by the daemon). `api.ts` is the typed client for `/api/*`;
`components.tsx` holds shared UI (`useAsync`, loading skeletons, run pickers).
Pages: Dashboard, Session Timeline, Patch Review, Command Risk, Token Spend,
Trace Analytics, Benchmarks, Rollback Points, Integration Status, and Ratify.
`npm run build` emits `apps/web/dist`, which the daemon embeds at compile time
(§9).

## 17. The landing site

`apps/landing/` — the public marketing site (React + Vite, browser-routed,
Tailwind). Pages under `src/pages/` (Home, About, DesktopDownload, CliDownload,
HostedDashboard, and a password-gated `Private` page that serves a testing
guide). `HostedDashboard` can connect to a visitor's own local daemon to show a
live preview. Install scripts (`install.sh`/`install.ps1`) are served from the
site's `public/` so the pipe-to-shell one-liner shows a trusted host.

## 18. The desktop app

`apps/desktop/` (crate `trace-desktop`, under `src-tauri`) — a Tauri shell that
bundles the daemon and dashboard into a native macOS/Windows/Linux app. It's a
packaging layer over the same daemon + web dashboard, not separate logic.

## 19. `trace-cloud-api` — optional cloud sync

`crates/trace-cloud-api/` — a **standalone** hosted service (it does *not*
depend on `trace-core`). If the user opts in, the daemon (`cloud_sync.rs`)
pushes **sanitized** run summaries to it for a hosted view across machines.
`auth.rs`, `routes.rs`, `db.rs`, `main.rs`. Deployed separately (see
`render.yaml`); the local product works fully without it.

## 20. The no-API-key guarantee

As of the deterministic refactor, **no part of Trace requires an AI/LLM API
key**. The command guard, secret scanner, policy engine, prompt-risk detector,
Ratify verdict, and both benchmarks are all pure pattern matching. The daemon
does not load provider keys, the config has no key fields, and no request path
calls a model provider. GitHub reads use a read-only token only for fetching PR
files/commits from `api.github.com` — never a model provider.

## 21. History: the TraceGuard → Trace → deterministic path

- **TraceGuard** was the original name; the project was renamed to **Trace**
  (binary `trace`; `traceguard` remained a temporary alias). The rule-pack env
  var followed: `TRACEGUARD_RULES_PATH` → `TRACE_RULES_PATH` (legacy honored).
- **Ratify** was once a separate Next.js service that reviewed PRs with a
  deterministic policy engine plus a single LLM reasoner. Its policy checks were
  ported into `trace-core/src/policy.rs`; the LLM reasoner became a 3-LLM
  "judge panel."
- A later change **removed all LLM features** — the judge panel, the prompting
  coach, and PR-history "doctrine" mining — so the product runs entirely on the
  deterministic engines with no key. "Ratify" now names the deterministic
  PR-review feature described in §14, integrated directly into the local
  dashboard.

## 22. Testing

- **Unit/integration tests** live beside the code (`#[cfg(test)]`), concentrated
  in `trace-core` (guard, secrets, policy, ratify, redteam, db, config, scan).
- **`trace self-check`** runs the labeled benchmarks against the real engines.
- **`cargo test --workspace`** runs everything; the frontends build with
  `npm run build` in `apps/web` and `apps/landing`.
- **CI** uses `trace review-diff --fail-on-risky` (the same engine) to gate
  risky changes on pull requests.

## 23. Data & privacy boundaries

- The daemon binds `127.0.0.1` only; the SQLite DB lives under `~/.trace`.
- Secret values are redacted at the point of detection — the raw value is never
  stored, logged, or returned over the API.
- The CI integration uploads **only** sanitized summaries (counts + finding
  titles/descriptions), never raw files, raw secrets, or the local database.
- GitHub tokens are read-only and only ever sent to `api.github.com`.
- Cloud sync is opt-in and pushes sanitized summaries only.

## 24. Complete surface reference

### CLI commands
`init`, `run`, `check`, `ratify`, `review-diff`, `self-check`, `scan`,
`dashboard`, `doctor`, `runs`, `show`, `patch`, `risks`, `costs`,
`checkpoints`, `replay`, `rollback`, `config {show,set}`,
`integrations {status,install}`, `github {status,commits,pulls,cat}`, `update`,
`daemon {start,stop,status}`, `__serve` (hidden).

### Daemon HTTP routes (all under `/api`, bound to `127.0.0.1`)
`GET /health`, `GET /state`, `GET /dashboard`,
`GET|POST /projects`, `GET /projects/:id`,
`GET|POST /runs`, `GET /runs/:id`, `POST /runs/:id/finish`,
`GET|POST /runs/:id/events`, `GET /runs/:id/timeline`, `GET /runs/:id/diff`,
`GET|POST /runs/:id/commands`, `GET|POST /runs/:id/secrets`,
`GET|POST /runs/:id/cost`, `POST /runs/:id/rollback`,
`POST /check-command`, `POST /scan`, `GET /doctor`, `GET /analytics`,
`GET /benchmarks`, `GET /benchmarks/redteam`,
`POST /runs/:id/analyze`, `POST /runs/:id/hook-check`, `GET /runs/:id/policy`,
`GET /github/status`, `GET /github/commits`, `GET /github/pulls`,
`GET /github/file`, `GET /github/ratify`.

### `trace-core` modules
`adapter`, `agents`, `config`, `cost`, `db`, `diagnose`, `eval`, `git`,
`github`, `guard`, `ids`, `models`, `paths`, `policy`, `prompt_quality`,
`ratify`, `redteam`, `rules_pack`, `scan`, `secrets`, `time`.
