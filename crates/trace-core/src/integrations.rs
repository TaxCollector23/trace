//! Agent integration detection — the single place that knows how to tell
//! whether Trace is actually wired into each supported coding agent's own
//! config, and what each integration's hook architecture can and cannot
//! enforce.
//!
//! Shared by `trc integrations status` (trace-cli) and the daemon's
//! `/api/integrations/coverage` route (`trace-daemon::health_routes`), so the
//! CLI and the dashboard can never disagree about what "connected" means —
//! there is exactly one detector.
//!
//! Two different kinds of fact live here, and they must not be confused:
//!
//!   * `connected` (via [`is_connected`] / [`detect_connections`]) is
//!     **live-detected** on this machine, right now, by reading the exact
//!     config file the installer patches (`hook_install.rs`).
//!   * `command_enforcement` / `file_review` on [`IntegrationDef`] are
//!     **structural capability facts** about each integration's own hook
//!     architecture — grounded in RECOVERY-AUDIT.md's "Agent integrations"
//!     table and the live end-to-end verification recorded there (2026-09).
//!     They do not change just because a config file is or isn't present;
//!     they describe what the hook can do *if* it is wired up.

use std::fs;
use std::path::PathBuf;

/// One supported agent integration: how to detect it, and what it can
/// actually enforce.
#[derive(Debug, Clone, Copy)]
pub struct IntegrationDef {
    /// Stable id — matches `trace_core::agents` ids and `runs.agent_name`.
    pub id: &'static str,
    pub display_name: &'static str,
    /// Short human description of the wiring, shown by `trc integrations status`.
    pub how: &'static str,
    /// Path (relative to `$HOME`) to the config file/dir that proves Trace is
    /// wired in, when it contains `needle` (or merely exists, for
    /// `existence_only` integrations that have no config file to patch).
    config_rel: &'static [&'static str],
    needle: &'static str,
    existence_only: bool,
    /// Can this integration's hook, once wired, actually BLOCK a dangerous
    /// shell command before it runs?
    /// `Some(true)`  — proven live (throws/denies/exits pre-exec).
    /// `Some(false)` — no command hook, or it is read-only/advisory only.
    /// `None`        — partial/unreliable: some invocations are classified,
    ///                  others are structurally invisible to the hook.
    pub command_enforcement: Option<bool>,
    /// Can this integration's hook actually block a risky file edit before
    /// it lands? Same tri-state meaning as `command_enforcement`.
    pub file_review: Option<bool>,
    /// One-line grounding for the two fields above, surfaced to the
    /// dashboard as `note` so "not instrumented" always comes with a reason.
    pub capability_note: &'static str,
}

/// The five agent integrations Trace ships today. Order matches the
/// RECOVERY-AUDIT.md scorecard.
pub const INTEGRATIONS: &[IntegrationDef] = &[
    IntegrationDef {
        id: "claude",
        display_name: "Claude Code",
        how: "PreToolUse + PostToolUse hooks",
        config_rel: &[".claude", "settings.json"],
        needle: "trace-hook",
        existence_only: false,
        command_enforcement: Some(true),
        file_review: Some(false),
        capability_note: "PreToolUse Bash hook blocks via exit 2 (verified live against /api/check-command). PostToolUse edit review is advisory-only: hook_check hardcodes block:false and only runs when TRACE_RUN_ID is set.",
    },
    IntegrationDef {
        id: "cursor",
        display_name: "Cursor",
        how: "MCP tools + enforcing guard hook",
        config_rel: &[".cursor", "mcp.json"],
        needle: ".trace/integrations/cursor",
        existence_only: false,
        command_enforcement: Some(true),
        file_review: Some(false),
        capability_note: "beforeShellExecution denies on block (verified live against /api/check-command). No file-edit hook exists.",
    },
    IntegrationDef {
        id: "windsurf",
        display_name: "Windsurf",
        how: "MCP server",
        config_rel: &[".codeium", "windsurf", "mcp_config.json"],
        needle: ".trace/integrations/windsurf",
        existence_only: false,
        command_enforcement: Some(false),
        file_review: Some(false),
        capability_note: "MCP server only — no beforeShellExecution-equivalent hook. Read-only/advisory even when connected; commands and edits are observed, never blocked.",
    },
    IntegrationDef {
        id: "codex",
        display_name: "Codex CLI",
        how: "wrapper script (add the shell alias to finish)",
        config_rel: &[".trace", "integrations", "codex", "codex-adapter.sh"],
        needle: "",
        existence_only: true,
        command_enforcement: None,
        file_review: Some(false),
        capability_note: "Only the top-level `codex …` invocation is classified via `trc run`; commands the agent runs as sub-processes are structurally invisible to the guard. FS/git are observed via the wrapper but never blocked.",
    },
    IntegrationDef {
        id: "opencode",
        display_name: "OpenCode",
        how: "MCP tools + enforcing guard plugin",
        config_rel: &[".config", "opencode", "opencode.json"],
        needle: ".trace/integrations/opencode",
        existence_only: false,
        command_enforcement: Some(true),
        file_review: None,
        capability_note: "tool.execute.before throws on block (verified live, pre-exec, fails open if the daemon is down). Edit review happens via trc run/MCP but blocking behavior for file edits is not independently verified.",
    },
];

fn config_text(rel: &[&str]) -> Option<String> {
    let mut p: PathBuf = dirs::home_dir()?;
    for seg in rel {
        p.push(seg);
    }
    fs::read_to_string(p).ok()
}

fn config_exists(rel: &[&str]) -> bool {
    let Some(mut p) = dirs::home_dir() else {
        return false;
    };
    for seg in rel {
        p.push(seg);
    }
    p.exists()
}

/// Real, live check: is Trace's hook/plugin actually wired into this agent's
/// own config file on this machine, right now? Mirrors exactly what the
/// installer patches, so this can never disagree with what was installed.
pub fn is_connected(def: &IntegrationDef) -> bool {
    if def.existence_only {
        return config_exists(def.config_rel);
    }
    config_text(def.config_rel)
        .map(|s| s.contains(def.needle))
        .unwrap_or(false)
}

/// Live connection state for one integration, in the shape `trc integrations
/// status` prints.
#[derive(Debug, Clone)]
pub struct AgentConnection {
    pub id: &'static str,
    pub display_name: &'static str,
    pub how: &'static str,
    pub connected: bool,
}

/// Detect live connection state for every supported integration.
pub fn detect_connections() -> Vec<AgentConnection> {
    INTEGRATIONS
        .iter()
        .map(|d| AgentConnection {
            id: d.id,
            display_name: d.display_name,
            how: d.how,
            connected: is_connected(d),
        })
        .collect()
}

/// Look up one integration's static definition by id.
pub fn by_id(id: &str) -> Option<&'static IntegrationDef> {
    INTEGRATIONS.iter().find(|d| d.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_ids_are_unique_and_match_agent_registry() {
        let ids: Vec<&str> = INTEGRATIONS.iter().map(|d| d.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "duplicate integration id");

        // Every integration id must resolve in the agents registry so
        // "installed?" detection (which keys off that registry) never misses.
        let registry = crate::agents::registry();
        for id in ids {
            assert!(
                registry.iter().any(|a| a.id == id),
                "integration id {id} has no matching trace_core::agents entry"
            );
        }
    }

    #[test]
    fn by_id_finds_known_and_rejects_unknown() {
        assert!(by_id("claude").is_some());
        assert!(by_id("not-a-real-agent").is_none());
    }

    #[test]
    fn detect_connections_covers_every_integration_without_panicking() {
        // This machine's actual home dir/config state is unknown to the test
        // (CI vs. a real dev box), so this only asserts the detector runs
        // cleanly end-to-end and returns one row per integration — never that
        // a specific agent is or isn't connected.
        let connections = detect_connections();
        assert_eq!(connections.len(), INTEGRATIONS.len());
    }
}
