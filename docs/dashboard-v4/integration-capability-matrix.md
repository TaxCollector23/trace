# Integration capability matrix

What each Trace agent integration can actually see and enforce, grounded in
`RECOVERY-AUDIT.md` ("Agent integrations") and the live end-to-end
verification run against the real daemon on 2026-09-02/03 (`/api/check-command`
proven to block `rm -rf /` for Claude Code and Cursor; the OpenCode plugin
source read and confirmed to throw on `block`; the Windsurf MCP registration
confirmed present with no accompanying command hook; Codex confirmed to have
no shell alias configured on that machine).

This is not a wishlist or a roadmap — it is what ships today. Where a cell
says "unavailable" or "partial", that is a structural limit of the
integration's own hook architecture, not a bug Trace can silently fix by
trying harder. The dashboard's `/api/integrations/coverage` endpoint
(`crates/trace-daemon/src/health_routes.rs`) and its underlying detector
(`crates/trace-core/src/integrations.rs`) encode exactly these facts — no
integration is ever shown "connected" without a live config-file check, and
no capability is ever shown "yes" without the grounding cited here.

Legend: ✅ enforced/full · 🟡 partial or observe-only (real signal, real gaps
named) · ⛔ unavailable (nothing to see, nothing to show).

| Integration | Timeline | Commands | Files | Policies | Processes | Tests | Network |
|---|---|---|---|---|---|---|---|
| **Claude Code** | ✅ full session/tool-call timeline via PreToolUse+PostToolUse hooks | ✅ blocks on `block` — PreToolUse Bash hook exits 2 (proven live against `/api/check-command`) | 🟡 observed, never blocked — PostToolUse `hook_check` hardcodes `block:false`; also only runs when `TRACE_RUN_ID` is set | 🟡 deterministic policy engine runs and records findings; `require_approval`/`warn` are advisory only (never enforced at the hook boundary) | ⛔ no process-tree capture in the schema or the hook | 🟡 recorded only when the wrapped command is itself a test runner (`test_results` table); no automatic test discovery | ⛔ no network-call capture |
| **Cursor** | ✅ MCP tool calls + `beforeShellExecution` events | ✅ blocks on `block` — `beforeShellExecution` denies (proven live) | ⛔ no file-edit hook exists at all | 🟡 same deterministic policy engine as Claude, same advisory-only ceiling for non-`block` decisions | ⛔ not captured | 🟡 same as Claude — only via an observed test-runner command | ⛔ not captured |
| **OpenCode** | ✅ plugin-level `tool.execute.before`/MCP timeline | ✅ blocks on `block` — plugin throws a real exception pre-exec (proven live); fails open if the daemon is down | 🟡 edits flow through `trc run`/MCP and are recorded, but blocking behavior for file edits is **not independently verified** — treat as advisory until proven otherwise | 🟡 same deterministic policy engine; same advisory-only ceiling | ⛔ not captured | 🟡 same test-runner-command caveat as above | ⛔ not captured |
| **Codex CLI** | 🟡 only the top-level `codex …` invocation is visible (via `trc run`'s wrapper) | 🟡 **partial** — the top-level invocation is classified, but commands the agent runs as sub-processes are structurally invisible to the guard (no in-agent hook, unlike Claude/Cursor/OpenCode) | 🟡 filesystem/git changes observed via the wrapper's diff capture, never blocked | 🟡 policy engine runs over what the wrapper captured; nothing beyond that is visible | ⛔ not captured | 🟡 same test-runner-command caveat | ⛔ not captured |
| **Windsurf** | 🟡 MCP server registered and queryable, but nothing drives it automatically — no hook triggers a timeline entry per action | ⛔ **no command hook of any kind** — no `beforeShellExecution` equivalent exists in this integration. Even when the MCP server is connected, commands are neither observed nor blocked | ⛔ MCP is read-only; no edit hook | ⛔ nothing to run the policy engine against | ⛔ not captured | ⛔ not captured | ⛔ not captured |

## Reading the table correctly

- **"Connected" ≠ "enforcing."** A live config-file check (does `~/.cursor/mcp.json`
  mention `.trace/integrations/cursor`? does `~/.claude/settings.json` mention
  `trace-hook`?) only proves Trace's hook/plugin is *wired in*. Whether that
  hook can actually stop something is a separate, static fact about the
  integration's own architecture — see `command_enforcement` / `file_review`
  in `/api/integrations/coverage`, which are `true`/`false`/`null`
  (never fabricated as `true` when the mechanism doesn't exist).
- **Windsurf is the clearest "connected but toothless" case.** Its MCP server
  can be fully registered and Trace will correctly report `connected: true`,
  while `command_enforcement` and `file_review` both report `false` — the
  dashboard must never render Windsurf with the same "protected" badge as
  Claude or Cursor.
- **Codex is the clearest "partial" case**, hence `command_enforcement: null`
  rather than `true` or `false` for it specifically: some commands genuinely
  are classified (the top-level invocation), others genuinely are not
  (anything the agent runs underneath it). Neither `true` nor `false` alone
  describes that honestly.
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
