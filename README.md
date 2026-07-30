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
gives you a reliable, local record of those actions, a deterministic rules
engine that catches the obvious problems instantly, and — when you want a
second opinion on a judgment call — a panel of three independent models that
vote separately and can tell the agent to stop and fix something before you
ever see it. It does not replace your agent and it does not write code for
you — it watches, judges, and lets you undo.

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
| `trace review-diff [--range] [--judge] [--fail-on-risky] [--json]` | Review a diff range with the policy engine (and, optionally, the judge panel) — no daemon or `trace init` required. What CI uses. |
| `trace self-check` | Run the policy engine's own labeled-fixture benchmark and print precision/recall. |
| `trace update` | Update the `trace` binary to the latest release. |
| `trace daemon start \| stop \| status` | Manage the local daemon. |
| `trace --help` / `trace --version` | Help and version. |

## Local dashboard

Served by the Rust daemon at `http://127.0.0.1:<port>` (prefers `8757`, falls
back to the next free port). Sections: **Dashboard** (with a live status strip
for the judge panel, prompting coach, and benchmark), **Run Timeline**,
**Patch Review** (with a Git diff view), **Cost Center**, **Risk Center**
(command decisions, policy findings, judge verdicts), **Judge Panel** (3-model
settings, per-slot connection testing, doctrine mining), **Prompting Coach**,
**Benchmarks**, **Rollback Center**, and **GitHub** status. It has clean
empty/loading/error states, status badges, and a clear local-only notice. No
fake data in production.

## Features

- Complete execution trace: prompts, commands, file changes, and cost tied to
  one run
- Agent run timeline and Git checkpointing
- Patch review grounded in the real Git diff
- File-change tracking with a debounced watcher
- Rule-based command guarding (allow / warn / require approval / block)
- **Deterministic policy engine** — secret detection, missing tests on
  sensitive paths, swallowed errors, direct DB access in handlers,
  hardcoded localhost, dependency/lockfile changes, and more, with a real
  labeled-fixture benchmark (`trace self-check` / the dashboard's
  **Benchmarks** page) proving precision and recall rather than asserting it
- **3-LLM judge panel** — three independently-configured models (any
  OpenAI-compatible provider works: Anthropic, OpenAI, Google, DeepSeek,
  Mistral, xAI, Groq, local Ollama, …) vote and reason together on judgment
  calls the deterministic rules can't settle alone. Can only escalate
  caution on top of the rule-based guard, never override a block. Retries
  once on a transient failure; a per-run cooldown caps API spend from rapid
  save loops.
- **Model Prompting Mode** — when on, a `require_approval`/`block` verdict is
  sent back to the coding agent as corrective feedback (full support for
  Claude Code via its hook system; other wrapped agents get a live terminal
  alert plus a dashboard flag, since they don't expose an equivalent hook
  surface yet). When off, verdicts are recorded to the dashboard only — the
  existing rollback path is how you undo something that slipped through.
- **Doctrine mining** — learns the rules a repo's reviewers actually enforce
  from its own merged-PR comment history, and weighs violations of those
  more heavily than generic best practices in the judge panel's reasoning
- **Prompting coach** — scores every prompt sent to an agent for clarity
  (too short, vague, conflicting, no acceptance criteria, well-scoped) and
  surfaces the recurring habits worth fixing
- **CI review** — `trace review-diff` runs the same policy+judge engine from
  a bare git checkout, no daemon needed; the GitHub Action installs and runs
  it automatically
- Local secret detection with redaction (raw secrets are never stored)
- Cost tracking with honest partial-data handling
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
- **GitHub** — App + Action running the real policy+judge engine
  (`trace review-diff`) and posting sanitized summaries (`integrations/github`)

GUI tools are observed via file changes and Git diffs; full command guarding
requires supported hooks or running through `trace run`.

## Architecture

- **CLI** (`trace`) handles user commands, runs the wrapped command, and hosts
  the daemon in a detached child process.
- **Daemon** (Axum) owns the local API, SQLite persistence, and dashboard
  serving — bound to `127.0.0.1` only.
- **Core** holds shared models, guard rules, the deterministic policy engine,
  the 3-LLM judge panel, doctrine mining, prompting analytics, secret rules,
  cost estimation, git inspection, and the database schema.
- **Web** is visualization only.
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
  wildcard — a permissive policy on a localhost daemon holding provider keys
  and able to trigger rollbacks would let *any webpage the user has open*
  script requests against it. Same-origin requests (the packaged app itself)
  don't need a CORS grant at all.
- Raw secrets are never stored; detected secrets are redacted.
- No cloud upload by default; project data stays on your machine.
- **Judge provider keys**: two modes. `OwnKeys` — keys read from
  `~/.trace/global.toml` or `TRACE_<PROVIDER>_API_KEY` env vars, sent only
  directly to that provider, never bundled into the app or sent to Trace's
  own servers. `BackendProxy` — a Trace-hosted endpoint holds the keys and
  meters usage instead. Keys are redacted before any config is sent to the
  dashboard UI.
- **Doctrine mining** uses the same read-only GitHub token as the `github`
  commands (env var / `gh` CLI / config file) to read a repo's own PR review
  history — nothing is written back to GitHub, and mining is only ever
  triggered explicitly from the dashboard, never automatically.
- GitHub integration uploads only sanitized summaries — never raw files,
  secrets, or the local SQLite database.
- Git is required for reliable checkpoints and rollback.

See [docs/security-model.mdx](docs/security-model.mdx).

## Limitations

- Full command guarding and run attribution require the `trace run` wrapper or
  supported hooks. GUI tools are otherwise observed only via file changes and
  Git diffs.
- **Live agent feedback** (the judge panel telling an agent to stop and fix
  something mid-session) is only fully wired for Claude Code, which has a
  hook system Trace can inject into. Other wrapped agents (Cursor, Aider,
  Codex, OpenCode, Gemini) get the same live policy+judge review, but a
  block surfaces as a terminal alert to the human plus a dashboard flag —
  not something the agent itself sees — because those tools don't expose an
  equivalent hook surface yet.
- The judge panel costs real API calls per provider; Model Prompting Mode
  trades edit latency for that live-review capability, and is off by
  default for that reason. A 15-second per-run cooldown caps spend from
  rapid save loops.
- Doctrine mining reads a repo's PR review history via a personal/CLI token,
  not a GitHub App — fine for local, user-triggered mining; an org-wide
  automatic version would need the App infrastructure this doesn't include.
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
- GitHub App-based doctrine mining for org-wide, automatic rule updates
- Transparent local cost proxy for live token capture
- Signed release artifacts and notarized desktop builds
- Optional (explicitly requested) enterprise features: org policy files, central
  policy management, team dashboards, SSO, audit exports

## License

MIT
