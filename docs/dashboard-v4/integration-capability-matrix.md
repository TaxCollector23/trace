# Integration capability matrix

What each Trace agent integration can actually see and enforce, grounded in
`RECOVERY-AUDIT.md` ("Agent integrations") and the live end-to-end
verification run against the real daemon on 2026-09-02/03 (`/api/check-command`
proven to block `rm -rf /` for Claude Code and Cursor; the OpenCode plugin
source read and confirmed to throw on `block`; the Windsurf MCP registration
confirmed present with no accompanying command hook). Claude Code and Cursor now
use lifecycle hooks for automatic runs; OpenCode supports both its V1 and V2
plugin contracts. Codex lifecycle hooks are installed through
`~/.codex/hooks.json` and must be trusted in `/hooks` before enforcement begins.

This is not a wishlist or a roadmap — it is what ships today. Where a cell
says "unavailable" or "partial", that is a structural limit of the
integration's own hook architecture, not a bug Trace can silently fix by
trying harder. The dashboard's read-only `/api/connections` endpoint
(`crates/trace-daemon/src/health_routes.rs`) and `trc doctor agents` share the
source-aware detector in `crates/trace-core/src/connections.rs`. The older
`/api/integrations/coverage` endpoint remains the hook/activity view. No
integration is ever shown "connected" without a live config-file check, and
no capability is ever shown "available" without the grounding cited here.

Legend: ✅ enforced/full · 🟡 partial or observe-only (real signal, real gaps
named) · ⛔ unavailable (nothing to see, nothing to show).

| Integration | Timeline | Commands | Files | Policies | Processes | Tests | Network |
|---|---|---|---|---|---|---|---|
| **Claude Code** | ✅ SessionStart/End + PreToolUse/PostToolUse timeline | ✅ PreToolUse denies `block` before Bash runs; fails closed if Trace is unavailable | 🟡 edits are recorded and reviewed after they land; they are not prevented before the tool call | 🟡 deterministic policy engine runs; only `block` is enforced | ⛔ no process-tree capture | 🟡 only when a test runner command is observed | ⛔ no network-call capture |
| **Cursor** | ✅ session, shell, and file-edit hooks | ✅ `beforeShellExecution` denies `block` before the shell starts; fails closed if Trace is unavailable | 🟡 `afterFileEdit` records and reviews the edit after it lands | 🟡 deterministic policy engine runs; only `block` is enforced | ⛔ no process-tree capture | 🟡 only when a test runner command is observed | ⛔ no network-call capture |
| **OpenCode** | ✅ V1/V2 tool hooks create a run and record completions | ✅ `execute.before`/`tool.execute.before` throws before Bash runs; fails closed if Trace is unavailable | 🟡 edits trigger capture/review after they land | 🟡 deterministic policy engine runs; only `block` is enforced | ⛔ no process-tree capture | 🟡 only when a test runner command is observed | ⛔ no network-call capture |
| **Codex** | ✅ SessionStart/End + Bash/apply_patch lifecycle events via `~/.codex/hooks.json` | ✅ `PreToolUse` denies dangerous Bash before execution after the hook is trusted; fails closed if the daemon is unavailable | 🟡 file edits are captured and reviewed after the call, not denied before it | 🟡 deterministic policy engine runs over captured commands and the final diff | ⛔ not captured | 🟡 same test-runner-command caveat | ⛔ not captured |
| **Windsurf** | 🟡 MCP server registered and queryable, but nothing drives it automatically — no hook triggers a timeline entry per action | ⛔ **no command hook of any kind** — no `beforeShellExecution` equivalent exists in this integration. Even when the MCP server is connected, commands are neither observed nor blocked | ⛔ MCP is read-only; no edit hook | ⛔ nothing to run the policy engine against | ⛔ not captured | ⛔ not captured | ⛔ not captured |

## Usage and connection evidence

The Connections page and `trc doctor agents` also report the local source
format and accounting capabilities. These are format claims, not a promise
that the current machine has an installed agent or imported rows.

| Agent | Tokens | Cost | Subagents | Tool calls | Command guard | Local source |
|---|---|---|---|---|---|---|
| **Claude Code** | available from local importer | partial; provider-reported/verified only | available in documented telemetry/import fields | available through hooks | available | `~/.claude/projects/` |
| **Codex** | available from rollout `token_usage_record` | not available in inspected local rollout files | partial across rollout versions | available in rollout/hook events | available after hook trust | `$CODEX_HOME/sessions/` or `~/.codex/sessions/` |
| **OpenCode** | available from assistant SQLite metadata | partial; reported field preserved, missing values unavailable | available from parent sessions | available from message/part metadata | available through plugin | `opencode.db` under the documented data root |
| **Cursor** | not available from inspected native/transcript files | not available | not available | partial | available through hooks | `~/.cursor/projects/**/agent-transcripts/` plus opaque `store.db` |
| **Aider** | partial; optional Markdown summary lines | partial; optional explicit summary lines | not available | partial; no stable Markdown tool schema | partial; wrapper only | `.aider.chat.history.md` |
| **Windsurf** | not available | not available | not available | partial through MCP | not available | MCP config only; no supported local usage importer |

`not available` means Trace has no evidence for that field in the supported
source format. It is rendered as `not available for this agent`, never as a
zero. Format-specific evidence and unresolved limitations live in
[`docs/formats/`](../formats/) and the OTel decision is recorded in
[`docs/otel-evaluation.md`](../otel-evaluation.md).

## Reading the table correctly

- **"Connected" ≠ "enforcing."** A live config-file check (does
  `~/.cursor/hooks.json` mention `agent-hook.js`? does `~/.claude/settings.json`
  mention the shared hook?) only proves Trace's hook/plugin is *wired in*. Whether that
  hook can actually stop something is a separate, static fact about the
  integration's own architecture — see `command_enforcement` / `file_review`
  in `/api/integrations/coverage`, which are `true`/`false`/`null`
  (never fabricated as `true` when the mechanism doesn't exist).
- **Windsurf is the clearest "connected but read-only" case.** Its MCP server
  can be fully registered and Trace will correctly report `connected: true`,
  while `command_enforcement` and `file_review` both report `false` — the
  dashboard must never render Windsurf with the same "protected" badge as
  Claude or Cursor.
- **Codex has real command enforcement once its hook is trusted.** Its
  `PreToolUse` Bash hook covers the commands Codex is about to run, while its
  file-edit lifecycle is post-call review only. The dashboard must show those
  as separate capabilities rather than one vague "connected" badge.
- **"require_approval" and "warn" are advisory everywhere.** Per
  RECOVERY-AUDIT.md's governing fact: *only `block` is enforced anywhere* —
  a `git reset --hard HEAD~1`-class command classifies `require_approval` and
  still runs on every integration in this table. Nothing in this matrix
  claims otherwise.
- **Process trees and network calls are unavailable across every
  integration** because no table in Trace's schema captures them today
  (`crates/trace-core/src/db.rs`'s `SCHEMA` has no such tables). This shows
  up in `/api/integrations/coverage` as `process_tree: {"status":
  "not_instrumented"}` for every agent, not as a fabricated "none observed."
- **Tests** are only visible when the wrapped command *is* a test runner
  (`trc run pytest`, etc.) and results are recorded through `test_results`.
  There is no automatic test-suite discovery for any integration, so this is
  marked 🟡 rather than ✅ everywhere.

## Source of truth in code

- Detection + capability facts: `crates/trace-core/src/integrations.rs`
  (`IntegrationDef`, `is_connected`, `detect_connections`) — the single
  detector shared by `trc integrations status` and the daemon.
- Real per-agent telemetry counts: `Store::agent_activity` in
  `crates/trace-core/src/db.rs`.
- HTTP surface: `crates/trace-daemon/src/health_routes.rs`
  (`GET /api/integrations/coverage`).
