# Trace

**Git stores commits. Trace stores execution.**

AI agents now write, run, and ship code. Trace gives every one of those
sessions a complete, local record — what changed, what ran, what it cost, what
looked risky, and how to roll it back — so "can we trust this change?" has an
answer instead of a guess.

It is **local-first**: the daemon and dashboard bind only to `127.0.0.1`, there
is no account, and your project data stays on your machine by default.

- **Landing:** https://landing-one-hazel-88.vercel.app (Vercel project `landing`; a cleaner `trace`-branded domain can be aliased later — see [Deploying the public surfaces](#deploying-the-public-surfaces))
- **Docs:** https://taxcollector23.github.io/trace/ (GitHub Pages, built from `/docs` via `apps/docs`)
- **Repo:** https://github.com/TaxCollector23/trace

## Why it exists

AI agents can edit files, run commands, touch secrets, change dependencies,
break builds, and spend API money — often faster than you can follow. Trace
gives you a reliable, local record of those actions and a **deterministic**
detection engine — a command guard, a secret scanner, and a policy engine —
that catches the obvious problems instantly. It's pure pattern matching, so it
runs in microseconds and needs **no AI/LLM API key**. It does not replace your
agent and it does not write code for you — it watches, flags, and lets you undo.

## Install

**macOS (Homebrew, recommended)**
```bash
brew tap TaxCollector23/trace
brew install trace
```

**Linux / macOS (shell script)**
```bash
curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh
```

**Windows (PowerShell)**
```powershell
irm https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.ps1 | iex
```

**npm (optional fallback)**
```bash
npm install -g trace
```

All methods install a single `trace` binary.

## Quickstart

```bash
trace init
trace run claude
trace dashboard
```

The dashboard opens locally at `http://127.0.0.1:<port>` (binds to `127.0.0.1`
only). Roll back a run's changes with `trace rollback`.

## CLI commands

| Command | Description |
| --- | --- |
| `trace init` | Initialize Trace in the current project. |
| `trace run <command>` | Run a command under monitoring (`--no-checks`, `-y`). |
| `trace dashboard` | Start the daemon if needed and open the dashboard. |
| `trace rollback` | Restore a checkpoint (Git-based, confirmed; `-y`). |
| `trace scan` | Detect the project's stack (package manager, framework, tests, git). |
| `trace doctor` | System checks: toolchain, clipboard, daemon, agents, paths. |
| `trace runs` | List recent runs. |
| `trace show <run_id>` | Show a run's summary and timeline. |
| `trace replay <run_id>` | Replay a run's events, commands, and file changes in order, paced by their real timestamps (`-y`/`--fast` to skip pacing). |
| `trace patch <run_id>` | Show the changed files for a run. |
| `trace risks <run_id>` | Show guarded commands and secret warnings for a run. |
| `trace costs <run_id>` | Show API usage and estimated cost for a run. |
| `trace checkpoints` | List checkpoints across recent runs. |
| `trace config show \| set` | View or change project configuration. |
| `trace integrations [status]` | List integrations or check what is live. |
| `trace github <status\|commits\|pulls\|cat>` | Read directly from the repo (incl. private). |
| `trace check <file>` | Run a file (or `-` for stdin) through the command guard + secret scanner; non-zero exit on require_approval/block. A CI gate for scripts. |
| `trace ratify <pr> [--fail-on-risky]` | Ratify a GitHub pull request against the policy engine — no daemon, no `trace init`, no API key. |
| `trace review-diff [--range] [--fail-on-risky] [--json]` | Review a diff range with the policy engine — no daemon or `trace init` required. What CI uses. |
| `trace self-check` | Run the policy + red-team benchmarks against the real engines and print recall / precision. |
| `trace update` | Update the `trace` binary to the latest release. |
| `trace daemon start \| stop \| status` | Manage the local daemon. |
| `trace --help` / `trace --version` | Help and version. |

## Local dashboard

Served by the Rust daemon at `http://127.0.0.1:<port>` (prefers `8757`, falls
back to the next free port). Sections: **Dashboard** (with a live status strip
for the policy + red-team benchmarks and Ratify), **Session Timeline**,
**Patch Review** (with a Git diff view), **Token Spend**, **Command Risk**
(command decisions, policy findings, secret warnings), **Trace Analytics**,
**Benchmarks**, **Rollback Points**, **Integration Status**, and **Ratify**
(deterministic PR review against a connected GitHub repo). It has clean
empty/loading/error states, status badges, and a clear local-only notice. No
fake data in production.

## Features

- Complete execution trace: prompts, commands, file changes, and cost tied to
  one run
- Agent run timeline and Git checkpointing
- Patch review grounded in the real Git diff
- File-change tracking with a debounced watcher
- **Rule-based command guarding** (allow / warn / require approval / block) —
  understands evasions the naive blocklist misses: pipe-to-shell
  (`curl … | sudo bash`), download-then-exec, base64-decoded payloads,
  `find … -delete`, raw block-device writes, DB drops
- **Deterministic policy engine** — secret detection, missing tests on
  sensitive paths, swallowed errors, hardcoded localhost, dependency/lockfile
  changes, and more, with a real labeled-fixture benchmark (`trace self-check`
  / the dashboard's **Benchmarks** page) proving precision and recall rather
  than asserting it. **No API key required — it's pure pattern matching.**
- **Red-team benchmark** — an adversarial corpus (dangerous commands incl.
  evasions, planted API keys, unsafe prompts) run through the *real* engines
  and scored for recall and false positives, surfaced in `trace self-check`,
  the API, and the dashboard
- **Ratify** — run the policy engine over a GitHub pull request's files and get
  a `pass` / `review` / `block` verdict, from the dashboard, `trace ratify <pr>`,
  or the API. No key.
- **`trace check`** — scan any file or script through the guard + secret
  scanner without executing it; non-zero exit for CI gating
- **CI review** — `trace review-diff` runs the same deterministic engine from a
  bare git checkout, no daemon needed; the GitHub Action installs and runs it
  automatically
- Local secret detection with redaction (raw secrets are never stored)
- Cost/token tracking with honest partial-data handling
- Build/test result recording and deterministic failure diagnosis
- Rollback center backed by Git
- Local SQLite history
- **GitHub repo reading** — reads commits, PRs, and file contents directly from
  your repo (including private repos) via an authenticated, read-only token
- Cross-platform desktop shell (macOS, Windows, Linux)

## Integrations

Each lives under `integrations/` and connects to the local daemon.

- **Claude Code** — hooks adapter with wrapper fallback (`integrations/claude`)
- **Codex** — CLI wrapper adapter (`integrations/codex`)
- **Cursor** — MCP server exposing Trace tools (`integrations/cursor`)
- **VS Code** — lightweight extension bridging to the daemon (`integrations/vscode`)
- **GitHub** — App + Action running the deterministic policy engine
  (`trace review-diff`) and posting sanitized summaries (`integrations/github`)

GUI tools are observed via file changes and Git diffs; full command guarding
requires supported hooks or running through `trace run`.

## Architecture

- **CLI** (`trace`) handles user commands, runs the wrapped command, and hosts
  the daemon in a detached child process.
- **Daemon** (Axum) owns the local API, SQLite persistence, and dashboard
  serving — bound to `127.0.0.1` only.
- **Core** holds shared models, guard rules, the deterministic policy engine,
  the secret scanner, the Ratify verdict logic, the benchmark corpora, cost
  estimation, git/GitHub inspection, and the database schema. All deterministic.
- **Web** is visualization only.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full, module-by-module reference.
- **SQLite** (`~/.trace/trace.db`) stores local history.
- **Git** provides checkpoints and rollback.

## Repository structure

```
trace
├─ README.md
├─ docs/                  # Markdown docs (single source)
├─ crates/
│  ├─ trace-cli/          # Rust CLI (binary: trace; hosts the daemon)
│  ├─ trace-daemon/       # Rust local API + localhost GUI server
│  └─ trace-core/         # shared logic
├─ apps/
│  ├─ web/                # React + Vite local dashboard
│  ├─ desktop/            # native desktop shell (Tauri) wrapping the dashboard
│  ├─ landing/            # public landing site (Vercel)
│  └─ docs/               # docs site → GitHub Pages (renders /docs)
├─ integrations/
│  ├─ github/             # GitHub App / Actions
│  ├─ vscode/             # VS Code extension
│  ├─ cursor/             # Cursor MCP server
│  ├─ claude/             # Claude Code hooks adapter
│  └─ codex/              # Codex CLI adapter
├─ packages/
│  └─ npm/                # optional npm wrapper package
├─ homebrew-trace/        # Homebrew formula (mirrors the tap repo)
├─ firebase/              # optional static mirror / reserved identity
├─ scripts/               # install.sh, install.ps1
└─ .github/workflows/     # release.yml, landing-deploy.yml, ci.yml
```

## Development setup

```bash
# Rust
cargo build
cargo test

# Dashboard (embedded by the daemon in release builds)
cd apps/web && npm install && npm run dev   # or: npm run build

# Landing site
cd apps/landing && npm install && npm run dev

# Run the daemon / CLI directly
cargo run -p trace-daemon
cargo run -p trace-cli -- --help
```

## Deploying the public surfaces

The public surfaces are marketing/onboarding only and never touch the local
daemon.

- **Vercel (landing):** deployed from `apps/landing` (Root Directory
  `apps/landing`, build command `npm run build`, output `dist`) to the
  `landing` project at https://landing-one-hazel-88.vercel.app. Set
  `VITE_MINTLIFY_DOCS_URL` to the deployed docs URL. Alias a `trace`-branded
  custom domain to this project when one is available.
- **Firebase (optional mirror):** `firebase/` hosts a static reserved page only.
  Do not add Auth/Firestore/Storage/Functions for the MVP.
- **GitHub Pages (docs):** `apps/docs` builds a fancy site from `/docs` and the
  `docs-deploy.yml` workflow publishes it to
  `https://taxcollector23.github.io/trace/`. Override the landing's docs
  link with `VITE_DOCS_URL` if needed.
- **Homebrew tap:** the formula lives in `homebrew-trace/Formula/trace.rb`,
  mirrored to `TaxCollector23/homebrew-trace` (tapped as
  `TaxCollector23/trace`, Homebrew's `homebrew-` prefix is implicit).

## Security model

- The local daemon binds only to `127.0.0.1` — never the local network.
- CORS is an explicit origin allow-list (the Vite dev server only), not a
  wildcard — a permissive policy on a localhost daemon able to trigger
  rollbacks would let *any webpage the user has open* script requests against
  it. Same-origin requests (the packaged app itself) don't need a CORS grant.
- **No AI/LLM API key is ever required or stored.** Every detection is
  deterministic pattern matching; no request path calls a model provider.
- Raw secrets are never stored; detected secrets are redacted at the point of
  detection.
- No cloud upload by default; project data stays on your machine.
- **GitHub token** (for `trace github`, `trace ratify`, and the CI action) is
  read-only, resolved from an env var / the `gh` CLI / `~/.trace/github.json`,
  and only ever sent to `api.github.com`. Nothing is written back to GitHub.
- GitHub integration uploads only sanitized summaries — never raw files,
  secrets, or the local SQLite database.
- Git is required for reliable checkpoints and rollback.

See [docs/security-model.mdx](docs/security-model.mdx).

## Limitations

- Full command guarding and run attribution require the `trace run` wrapper or
  supported hooks. GUI tools are otherwise observed only via file changes and
  Git diffs.
- **Live agent feedback** (surfacing a finding back into the agent's own
  context mid-session) is only fully wired for Claude Code, which has a hook
  system Trace can inject into. Other wrapped agents (Cursor, Aider, Codex,
  OpenCode, Gemini) get the same live policy review, but findings surface as a
  terminal alert to the human plus a dashboard flag — not something the agent
  itself sees — because those tools don't expose an equivalent hook surface yet.
- Ratify reads a repo's PR files via a personal/CLI token, not a GitHub App —
  fine for local, user-triggered ratification; an org-wide automatic version
  would need App infrastructure this doesn't include.
- Cost is "unavailable" unless usage is routed through a proxy or imported.
- Rollback requires a Git repository.

See [docs/limitations.mdx](docs/limitations.mdx).

## Engineering decisions

- **UI library:** the local dashboard uses a small custom CSS design system (no
  runtime UI dependency). It is already a serious developer-tool layout;
  Tailwind + shadcn/ui can be adopted later without changing the API.
- **Diff viewer:** a custom unified-diff renderer (no heavy dependency).
- **Desktop shell:** Tauri wrapping the existing dashboard build — no second UI
  to maintain.
- **Analytics:** none on the landing site by default.
- **Code signing:** not day-one; macOS/Windows may warn on downloaded binaries
  until notarization/signing is added.
- **License:** MIT.

## Roadmap

- Live agent-feedback delivery for adapters beyond Claude Code, bounded by
  what each tool's own hook/plugin surface actually exposes
- GitHub App-based Ratify for org-wide, automatic PR checks
- Transparent local cost proxy for live token capture
- Signed release artifacts and notarized desktop builds
- Optional (explicitly requested) enterprise features: org policy files, central
  policy management, team dashboards, SSO, audit exports

## License

MIT
