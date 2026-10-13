# Trace

Your AI can code. Trace shows what it did.

Trace is free and open source. It watches AI coding sessions on your computer,
records commands and file changes, stops commands it recognizes as dangerous,
and gives you a local dashboard to review or undo the work.

Trace is local-first. The daemon and dashboard bind to `127.0.0.1`, there is no
account, and your project data stays on your machine by default.

- Landing page: https://trace-landing-blue.vercel.app
- Documentation: https://taxcollector23.github.io/trace/
- Source code: https://github.com/TaxCollector23/trace

## Install

Install the `trc` command on macOS, Linux, or Windows:

```bash
npm install -g trace-agent-cli
```

The install scripts verify the downloaded binary with SHA-256 before placing it
on your machine.

## Quick start

```bash
trc init
trc integrations install all
trc dashboard
```

Restart the agents you use. Trace starts the local dashboard and prints its
address in the terminal. You can also watch one command directly:

```bash
trc run "your agent command"
```

The dashboard is local and normally opens at `http://127.0.0.1:8757`.

## What Trace does

Trace turns every session into something you can see, stop, and undo:

- **A checkpoint before anything changes.** Trace saves your Git state before a
  run starts, so you have a safety net before the first edit lands.
- **Every file change as a diff you can read.** Review the real additions and
  deletions instead of trusting a summary.
- **A guard on every command.** Commands are classified as allow, warn, require
  approval, or block. Destructive commands are stopped before they run.
- **Secret scanning.** Keys, tokens, and credentials in a diff are caught and
  redacted before they are stored.
- **Usage and cost with provenance.** Imported token counts are grouped by
  model, session, day, agent, or project. Reported and computed costs stay
  distinct, and unavailable values remain visible.
- **Pull request checks.** Run the same checks over a pull request and get a
  clear `pass`, `review`, or `block` verdict.
- **One-click rollback.** Restore a checkpoint when a run went somewhere you
  did not want it to go.

Every safety check is deterministic pattern matching. There is no LLM in the
guard path and no API key is required, so the result is instant, free, and the
same on every machine. Trace includes labeled dangerous commands, planted
secrets, and unsafe prompts so you can reproduce the detection results.

## The local dashboard

Open it with:

```bash
trc dashboard
```

The dashboard shows the current recorded run first: which agent is working,
where it is working, file and command counts, secret warnings, and estimated
cost when available. It also explains whether agent integrations are configured
and links to the timeline, usage, safety decisions, benchmarks, and rollback.

The public landing page also has a `/dashboard` route, but that page is only a
browser frontend. It connects to a Trace daemon running on your own computer;
it does not host your data.

## CLI commands

### Start and inspect Trace

```text
trc init                         Set up Trace in the current project
trc run <command>                Run a command under Trace monitoring
trc dashboard                   Start the daemon and open the local dashboard
trc status [--json]              Show the local connection status
trc doctor                      Check the toolchain, daemon, agents, and paths
trc doctor agents [--json]      Show installed agents and integration evidence
trc integrations status         Show which integrations are live
trc integrations install all    Connect supported agents in one command
trc daemon start|stop|status    Manage the local daemon
```

### Review runs

```text
trc runs                         List recent runs
trc show <run-id>                Show a run summary and timeline
trc replay <run-id>              Replay events, commands, and file changes
trc patch <run-id>               Show changed files
trc risks <run-id>               Show command decisions and secret warnings
trc costs <run-id>               Show usage and estimated cost
trc checkpoints                  List saved rollback points
trc rollback                     Restore the latest checkpoint
```

### Usage and pricing

Trace imports local accounting metadata without importing prompts, responses,
tool arguments, or tool output content:

```bash
trc import claude --json
trc import codex --json
trc import cursor --json
trc import opencode --json
trc import aider --json
trc import all --json
trc usage --by model --json
trc usage --by day --since 2026-10-12 --json
trc verify --day 2026-10-12
```

`trc verify` places Trace’s local accounting beside the provider’s own usage
page. It does not pretend that an estimate is an invoice. Unknown models and
missing provider data stay unavailable instead of becoming zero.

Inspect or refresh the versioned pricing catalog with:

```bash
trc pricing show
trc pricing show --json
trc pricing update
```

### Connect API usage without storing content

The optional loopback proxy captures provider-reported token metadata while
passing requests through. It does not store prompts, responses, or API keys.

```bash
trc proxy openai
trc proxy anthropic
trc proxy google
trc proxy openai --run-id <run-id> --json
```

Trace’s dollar value is an API-equivalent price-table estimate, not an invoice
or subscription bill. The proxy is local and opt-in.

### Safety and CI

Scan a file or script without executing it:

```bash
trc check script.sh
cat script.sh | trc check -
```

Review a diff in CI, with no daemon and no API key:

```bash
trc review-diff --range origin/main...HEAD --fail-on-risky
trc ratify 123 --fail-on-risky
trc self-check --json
```

`trc self-check` runs the policy and red-team fixtures shipped with Trace.
Those fixtures cover dangerous commands, evasions, secrets, and unsafe prompts.

### Local data

Trace stores local history in SQLite under the active Trace home. To inspect
what a local reset would remove:

```bash
trc reset --local-data --dry-run
trc reset --local-data
```

Reset is explicit and asks for confirmation. It removes local telemetry, not
your project files or Git history.

## Supported agents

Trace shows what your coding agent did, what it changed, and whether it tried
something risky.

| Agent | What Trace can do |
| --- | --- |
| Claude Code | Hooks for sessions, commands, and edits; command enforcement; local usage import |
| Codex | Lifecycle hooks and wrapper fallback; command enforcement after hooks; usage import |
| Cursor | Hooks and MCP tools; command enforcement; optional local usage import |
| OpenCode | Guard plugin and MCP tools; command enforcement; local usage import |
| Aider | Local usage import; run through `trc run aider` for guarding |
| Windsurf | Activity and file changes through the read-only integration; no command hook |
| GitHub Copilot | File changes through the supported observation path |

GUI tools can always be observed through file changes and Git diffs. Full
command guarding requires a supported hook or running the agent through
`trc run`.

## Privacy and security

- The daemon binds to `127.0.0.1`, not the local network.
- Trace does not require an AI or LLM API key for its safety checks.
- Prompts, responses, tool arguments, and tool output are not imported by the
  usage importers.
- Secrets found in diffs are redacted before storage.
- Provider proxy requests are forwarded in memory; only usage metadata is
  captured.
- GitHub access is opt-in and uses the connection you provide.
- Cost values are estimates unless a provider reports the value directly.

Read the full policies in [`docs/privacy.md`](docs/privacy.md) and
[`SECURITY.md`](SECURITY.md).

## Repository layout

```text
crates/trace-core/       Shared models, policy rules, pricing, importers, and storage
crates/trace-daemon/     Local API, SQLite history, proxy, and dashboard server
crates/trace-cli/        The `trc` command
apps/web/                Local dashboard UI
apps/landing/            Public landing page
apps/docs/               Documentation site
integrations/            Agent and GitHub integrations
packages/npm/            npm wrapper and installers
experimental/            Retired desktop, cloud, hosting, and webhook sketches
scripts/                 Installer and verification scripts
```

The supported product is the local CLI, daemon, and browser dashboard.
Experimental directories are retained for reference and are not release
targets.

## Development

Requirements: Rust, Node.js, npm, and Git.

```bash
cargo build
cargo test --workspace
cargo fmt --all -- --check

cd apps/web
npm ci
npm run build

cd ../landing
npm ci
npm run build
```

Run the CLI directly during development:

```bash
cargo run -p trace-cli -- --help
cargo run -p trace-cli -- self-check
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md), [`ARCHITECTURE.md`](ARCHITECTURE.md),
and the [`docs/`](docs/) directory for project conventions and deeper guides.

## Public surfaces

- Landing page: Vercel deploy from `apps/landing`.
- Documentation: GitHub Pages from `apps/docs` and `/docs`.
- Releases: GitHub Actions builds the CLI and publishes checksums.

The public landing page is for explanation and onboarding. It does not receive
your local Trace data.

## License

See [`LICENSE`](LICENSE).
