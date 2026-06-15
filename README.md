# TraceGuard

**TraceGuard is a local black box recorder for AI coding agents.**

TraceGuard shows what AI coding agents changed, what they ran, what they cost,
what looked risky, and how to roll back — before your project turns into a
mystery. It is **local-first**: the daemon and dashboard run only on
`127.0.0.1`, there is no account, and your project data stays on your machine by
default.

- **Landing:** https://traceguardlanding.vercel.app *(deploy target; see [Deploying the public surfaces](#deploying-the-public-surfaces))*
- **Docs:** Mintlify (set `MINTLIFY_DOCS_URL` / `VITE_MINTLIFY_DOCS_URL` to the deployed URL)
- **Repo:** https://github.com/TaxCollector23/TraceGuard

## Why it exists

AI agents can edit files, run commands, touch secrets, change dependencies,
break builds, and spend API money — often faster than you can follow. TraceGuard
gives you a reliable, local record of those actions so you can review agent edits
before accepting them, get command-risk warnings, see cost, and roll back. It
does not replace your agent and it does not write code for you.

## Install

**macOS (Homebrew, recommended)**
```bash
brew tap TaxCollector23/traceguard
brew install traceguard
```

**Linux / macOS (shell script)**
```bash
curl -fsSL https://raw.githubusercontent.com/TaxCollector23/TraceGuard/main/scripts/install.sh | sh
```

**Windows (PowerShell)**
```powershell
irm https://raw.githubusercontent.com/TaxCollector23/TraceGuard/main/scripts/install.ps1 | iex
```

**npm (optional fallback)**
```bash
npm install -g traceguard
```

All methods install the `trg` binary and a `traceguard` alias.

## Quickstart

```bash
trg init
trg run claude
trg dashboard
```

The dashboard opens locally at `http://127.0.0.1:<port>` (binds to `127.0.0.1`
only). Roll back a run's changes with `trg rollback`.

## CLI commands

| Command | Description |
| --- | --- |
| `trg init` | Initialize TraceGuard in the current project. |
| `trg run <command>` | Run a command under monitoring (`--no-checks`, `-y`). |
| `trg dashboard` | Start the daemon if needed and open the dashboard. |
| `trg rollback` | Restore a checkpoint (Git-based, confirmed; `-y`). |
| `trg compress-prompt [text]` | Compress a prompt locally (`--budget`, `--target`, `-y`). |
| `trg daemon start \| stop \| status` | Manage the local daemon. |
| `trg --help` / `trg --version` | Help and version. |

## Local dashboard

Served by the Rust daemon at `http://127.0.0.1:<port>` (prefers `8757`, falls
back to the next free port). Sections: **Dashboard**, **Run Timeline**, **Patch
Review** (with a Git diff view), **Cost Center**, **Risk Center**, **Rollback
Center**, and **Utilities** (Prompt Compressor + output budget). It has clean
empty/loading/error states, status badges, and a clear local-only notice. No
fake data in production.

## Features

- Agent run timeline and Git checkpointing
- Patch review grounded in the real Git diff
- File-change tracking with a debounced watcher
- Rule-based command guarding (allow / warn / require approval / block)
- Local secret detection with redaction (raw secrets are never stored)
- Cost tracking with honest partial-data handling
- Build/test result recording and deterministic failure diagnosis
- Rollback center backed by Git
- Local SQLite history
- Prompt Compressor and output-budget controls

## Integrations

Each lives under `integrations/` and connects to the local daemon.

- **Claude Code** — hooks adapter with wrapper fallback (`integrations/claude`)
- **Codex** — CLI wrapper adapter (`integrations/codex`)
- **Cursor** — MCP server exposing TraceGuard tools (`integrations/cursor`)
- **VS Code** — lightweight extension bridging to the daemon (`integrations/vscode`)
- **GitHub** — App + Action posting sanitized summaries (`integrations/github`)

GUI tools are observed via file changes and Git diffs; full command guarding
requires supported hooks or running through `trg run`.

## Architecture

- **CLI** (`trg`) handles user commands, runs the wrapped command, and hosts the
  daemon in a detached child process.
- **Daemon** (Axum) owns the local API, SQLite persistence, and dashboard
  serving — bound to `127.0.0.1` only.
- **Core** holds shared models, guard rules, secret rules, cost estimation, git
  inspection, the Prompt Compressor, and the database schema.
- **Web** is visualization only.
- **SQLite** (`~/.traceguard/traceguard.db`) stores local history.
- **Git** provides checkpoints and rollback.

## Repository structure

```
TraceGuard
├─ README.md
├─ docs/                  # Mintlify docs
├─ crates/
│  ├─ trg-cli/            # Rust CLI (binary: trg; hosts the daemon)
│  ├─ traceguard-daemon/  # Rust local API + localhost GUI server
│  └─ traceguard-core/    # shared logic
├─ apps/
│  ├─ web/                # React + Vite local dashboard
│  └─ landing/            # public landing site (Vercel)
├─ integrations/
│  ├─ github/             # GitHub App / Actions
│  ├─ vscode/             # VS Code extension
│  ├─ cursor/             # Cursor MCP server
│  ├─ claude/             # Claude Code hooks adapter
│  └─ codex/              # Codex CLI adapter
├─ packages/
│  └─ npm/                # optional npm wrapper package
├─ homebrew-tap/          # Homebrew formula (mirrors the tap repo)
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
cargo run -p traceguard-daemon
cargo run -p trg-cli -- --help
```

## Deploying the public surfaces

The public surfaces are marketing/onboarding only and never touch the local
daemon.

- **Vercel (landing):** import the repo, set **Root Directory** to
  `apps/landing`, build command `npm run build`, output `dist`. Target URL
  `traceguardlanding.vercel.app` (use the closest available name and update this
  README). Set `VITE_MINTLIFY_DOCS_URL` to the deployed docs URL.
- **Firebase (optional mirror):** `firebase/` hosts a static reserved page only.
  Do not add Auth/Firestore/Storage/Functions for the MVP.
- **Mintlify (docs):** deploy `docs/` and point `MINTLIFY_DOCS_URL` at it.
- **Homebrew tap:** the formula lives in `homebrew-tap/Formula/traceguard.rb`,
  mirrored to `TaxCollector23/homebrew-traceguard`.

## Security model

- The local daemon binds only to `127.0.0.1` — never the local network.
- Raw secrets are never stored; detected secrets are redacted.
- No cloud upload by default; project data stays on your machine.
- GitHub integration uploads only sanitized summaries — never raw files,
  secrets, or the local SQLite database.
- The Prompt Compressor runs locally and deterministically; any future
  LLM-based compression is opt-in and clearly labelled.
- Git is required for reliable checkpoints and rollback.

See [docs/security-model.mdx](docs/security-model.mdx).

## Limitations

- Full command guarding and run attribution require the `trg run` wrapper or
  supported hooks. GUI tools are otherwise observed only via file changes and
  Git diffs.
- Cost is "unavailable" unless usage is routed through a proxy or imported.
- Rollback requires a Git repository.

See [docs/limitations.mdx](docs/limitations.mdx).

## Roadmap

- Transparent local cost proxy for live token capture
- Deeper Claude/Codex hook coverage
- Homebrew core formula and signed release artifacts
- Optional (explicitly requested) enterprise features: org policy files, central
  policy management, team dashboards, SSO, audit exports

## License

MIT
