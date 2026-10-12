//! Evidence-backed connection status shared by `trc doctor agents` and the
//! dashboard.
//!
//! A connection is not the same thing as an installed binary, and an agent's
//! capability is not inferred from an empty database row. This module keeps
//! those facts separate: it reports the executable/config/source evidence we
//! can see, the canonical events already imported, and a capability matrix
//! whose unavailable values are explicit rather than numeric zeroes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::agents;
use crate::importers::{self, ImportContext};
use crate::integrations;
use crate::Store;

/// Stable order used by the CLI and dashboard onboarding surfaces.
pub const SUPPORTED_AGENT_IDS: &[&str] =
    &["claude", "codex", "opencode", "cursor", "aider", "windsurf"];

/// Capability evidence level. `NotAvailable` means the agent/source does not
/// provide the field; `Unknown` means Trace has not verified it. Neither is a
/// fabricated zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Available,
    Partial,
    NotAvailable,
    Unknown,
}

impl CapabilityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Partial => "partial",
            Self::NotAvailable => "not_available",
            Self::Unknown => "unknown",
        }
    }
}

/// The dashboard-facing capability matrix for one supported coding agent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMatrix {
    pub tokens: CapabilityStatus,
    pub cost: CapabilityStatus,
    pub subagents: CapabilityStatus,
    pub tool_calls: CapabilityStatus,
    pub command_guard: CapabilityStatus,
}

/// One read-only connection/status row. Paths are local evidence and are only
/// returned to the local daemon/dashboard; no transcript contents are read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub connected: bool,
    pub session_paths: Vec<String>,
    pub session_data_found: bool,
    pub last_event_at: Option<String>,
    pub events_imported: i64,
    pub parse_errors: i64,
    pub capabilities: CapabilityMatrix,
    pub status: String,
    pub action: String,
    pub install_hint: Option<String>,
    pub note: String,
    pub importer: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct AgentSpec {
    id: &'static str,
    importer: Option<&'static str>,
    expected_rel: &'static [&'static str],
    usage_ids: &'static [&'static str],
    capabilities: CapabilityMatrix,
    note: &'static str,
}

const SPEC_CLAUDE: AgentSpec = AgentSpec {
    id: "claude",
    importer: None,
    expected_rel: &[".claude", "projects"],
    usage_ids: &["claude", "claude-code"],
    capabilities: CapabilityMatrix {
        tokens: CapabilityStatus::Available,
        cost: CapabilityStatus::Partial,
        subagents: CapabilityStatus::Available,
        tool_calls: CapabilityStatus::Available,
        command_guard: CapabilityStatus::Available,
    },
    note: "Local Claude Code transcripts expose usage metadata; cost is only available when the provider reports it or a separately verified price calculation exists.",
};

const SPEC_CODEX: AgentSpec = AgentSpec {
    id: "codex",
    importer: Some("codex"),
    expected_rel: &[".codex", "sessions"],
    usage_ids: &["codex"],
    capabilities: CapabilityMatrix {
        tokens: CapabilityStatus::Available,
        cost: CapabilityStatus::NotAvailable,
        subagents: CapabilityStatus::Partial,
        tool_calls: CapabilityStatus::Available,
        command_guard: CapabilityStatus::Available,
    },
    note: "Codex rollout JSONL has token usage and tool-event structure; local rollout records do not provide provider cost, and subagent linkage varies by rollout version.",
};

const SPEC_OPENCODE: AgentSpec = AgentSpec {
    id: "opencode",
    importer: Some("opencode"),
    expected_rel: &[".local", "share", "opencode"],
    usage_ids: &["opencode"],
    capabilities: CapabilityMatrix {
        tokens: CapabilityStatus::Available,
        cost: CapabilityStatus::Partial,
        subagents: CapabilityStatus::Available,
        tool_calls: CapabilityStatus::Available,
        command_guard: CapabilityStatus::Available,
    },
    note: "OpenCode's SQLite metadata includes token fields and a reported cost field; Trace preserves that field but does not turn a missing/opaque value into a price estimate.",
};

const SPEC_CURSOR: AgentSpec = AgentSpec {
    id: "cursor",
    importer: Some("cursor"),
    expected_rel: &[".cursor"],
    usage_ids: &["cursor"],
    capabilities: CapabilityMatrix {
        tokens: CapabilityStatus::NotAvailable,
        cost: CapabilityStatus::NotAvailable,
        subagents: CapabilityStatus::NotAvailable,
        tool_calls: CapabilityStatus::Partial,
        command_guard: CapabilityStatus::Available,
    },
    note: "Cursor's native chat store is opaque and local transcripts observed by Trace do not carry usage fields; tool-call detail is therefore partial.",
};

const SPEC_AIDER: AgentSpec = AgentSpec {
    id: "aider",
    importer: Some("aider"),
    expected_rel: &[".aider.chat.history.md"],
    usage_ids: &["aider"],
    capabilities: CapabilityMatrix {
        tokens: CapabilityStatus::Partial,
        cost: CapabilityStatus::Partial,
        subagents: CapabilityStatus::NotAvailable,
        tool_calls: CapabilityStatus::Partial,
        command_guard: CapabilityStatus::Partial,
    },
    note: "Aider's Markdown history may contain optional token/cost summaries; it has no stable local subagent/tool schema and Trace only guards commands when wrapped with `trc run`.",
};

const SPEC_WINDSURF: AgentSpec = AgentSpec {
    id: "windsurf",
    importer: None,
    expected_rel: &[".codeium", "windsurf", "mcp_config.json"],
    usage_ids: &["windsurf"],
    capabilities: CapabilityMatrix {
        tokens: CapabilityStatus::NotAvailable,
        cost: CapabilityStatus::NotAvailable,
        subagents: CapabilityStatus::NotAvailable,
        tool_calls: CapabilityStatus::Partial,
        command_guard: CapabilityStatus::NotAvailable,
    },
    note: "Windsurf is connected through an MCP server only; Trace has no documented local session/usage format or pre-execution command hook for this integration.",
};

const SPECS: &[AgentSpec] = &[
    SPEC_CLAUDE,
    SPEC_CODEX,
    SPEC_OPENCODE,
    SPEC_CURSOR,
    SPEC_AIDER,
    SPEC_WINDSURF,
];

fn path_from_parts(home: &Path, parts: &[&str]) -> PathBuf {
    let mut path = home.to_path_buf();
    for part in parts {
        path.push(part);
    }
    path
}

fn expected_path(context: &ImportContext, spec: &AgentSpec) -> PathBuf {
    match spec.id {
        "codex" => std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| context.home.join(".codex"))
            .join("sessions"),
        "opencode" => std::env::var_os("OPENCODE_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_DATA_HOME").map(|path| PathBuf::from(path).join("opencode"))
            })
            .unwrap_or_else(|| context.home.join(".local/share/opencode")),
        _ => path_from_parts(&context.home, spec.expected_rel),
    }
}

fn installed_map() -> Vec<agents::AgentStatus> {
    agents::detect_all()
}

fn usage_stats(store: &Store, ids: &[&str]) -> Result<(i64, Option<String>, i64)> {
    let mut count = 0;
    let mut last_event_at = None;
    let mut parse_errors = 0;
    for id in ids {
        count += store.usage_event_count(id)?;
        if let Some(candidate) = store.usage_event_last_ts(id)? {
            if last_event_at
                .as_deref()
                .map(|existing: &str| candidate.as_str() > existing)
                .unwrap_or(true)
            {
                last_event_at = Some(candidate);
            }
        }
        parse_errors += store
            .import_sources(Some(id))?
            .into_iter()
            .map(|source| source.parse_errors)
            .sum::<i64>();
    }
    Ok((count, last_event_at, parse_errors))
}

fn source_paths(context: &ImportContext, spec: &AgentSpec) -> Vec<PathBuf> {
    let mut paths = if let Some(importer_id) = spec.importer {
        importers::importer(importer_id)
            .map(|importer| importer.discover(context))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if paths.is_empty() {
        let expected = expected_path(context, spec);
        if expected.exists() {
            paths.push(expected);
        } else {
            paths.push(expected);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn action_for(
    spec: &AgentSpec,
    installed: bool,
    connected: bool,
    source_paths: &[PathBuf],
    source_found: bool,
    events_imported: i64,
    parse_errors: i64,
    install_hint: Option<&str>,
) -> String {
    if !installed && spec.id != "aider" {
        return format!(
            "{}; then run `trc doctor agents`",
            install_hint.unwrap_or("install this agent")
        );
    }
    if parse_errors > 0 {
        return format!(
            "{parse_errors} parse error(s) recorded; inspect the source and re-run `trc import {}`",
            spec.importer.unwrap_or("agent")
        );
    }
    if !source_found {
        let path = source_paths
            .first()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "the documented local session path".to_string());
        if let Some(importer) = spec.importer {
            return format!(
                "no sessions found at {path}; run {} once, then re-check with `trc import {importer}`",
                spec.id
            );
        }
        return format!(
            "no sessions found at {path}; run {} once, then re-check",
            spec.id
        );
    }
    if let Some(importer) = spec.importer {
        if events_imported == 0 {
            return format!("sessions found; run `trc import {importer}`, then re-check");
        }
    } else if spec.id == "claude" && events_imported == 0 {
        return "sessions found; run `trc usage`, then re-check".to_string();
    }
    if connected {
        return "connected; run one agent task to confirm the next event".to_string();
    }
    if spec.id == "aider" {
        return "history is available through the importer; command guarding requires `trc run`"
            .to_string();
    }
    if spec.importer.is_none() {
        return "Trace has no local session importer for this integration; MCP activity is read-only".to_string();
    }
    "session data is present but the Trace hook is not connected; run `trc integrations install`"
        .to_string()
}

/// Inspect all supported coding-agent connections without mutating agent
/// configuration or reading prompt/response content.
pub fn inspect(store: &Store, context: &ImportContext) -> Result<Vec<ConnectionStatus>> {
    let installed_agents = installed_map();
    let mut rows = Vec::with_capacity(SPECS.len());
    for spec in SPECS {
        let installed = installed_agents.iter().find(|agent| agent.id == spec.id);
        let source_paths = source_paths(context, spec);
        let source_data_found = source_paths.iter().any(|path| path.exists());
        let (events_imported, last_event_at, parse_errors) = usage_stats(store, spec.usage_ids)?;
        let connected = integrations::by_id(spec.id)
            .map(integrations::is_connected)
            .unwrap_or(false);
        let action = action_for(
            spec,
            installed.map(|agent| agent.installed).unwrap_or(false),
            connected,
            &source_paths,
            source_data_found,
            events_imported,
            parse_errors,
            installed.and_then(|agent| agent.install_hint.as_deref()),
        );
        rows.push(ConnectionStatus {
            id: spec.id.to_string(),
            name: installed
                .map(|agent| agent.name.clone())
                .unwrap_or_else(|| {
                    match spec.id {
                        "claude" => "Claude Code",
                        "codex" => "Codex CLI",
                        "opencode" => "OpenCode",
                        "cursor" => "Cursor",
                        "aider" => "Aider",
                        "windsurf" => "Windsurf",
                        _ => spec.id,
                    }
                    .to_string()
                }),
            installed: installed.map(|agent| agent.installed).unwrap_or(false),
            path: installed.and_then(|agent| agent.path.clone()),
            version: installed.and_then(|agent| agent.version.clone()),
            connected,
            session_paths: source_paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            session_data_found: source_data_found,
            last_event_at,
            events_imported,
            parse_errors,
            capabilities: spec.capabilities,
            status: if connected {
                "connected".to_string()
            } else if source_data_found {
                "data_found".to_string()
            } else if installed.map(|agent| agent.installed).unwrap_or(false) {
                "installed".to_string()
            } else {
                "not_installed".to_string()
            },
            action,
            install_hint: installed.and_then(|agent| agent.install_hint.clone()),
            note: spec.note.to_string(),
            importer: spec.importer.map(ToOwned::to_owned),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_order_and_unknown_values_are_stable() {
        assert_eq!(
            SUPPORTED_AGENT_IDS,
            ["claude", "codex", "opencode", "cursor", "aider", "windsurf"]
        );
        assert_eq!(CapabilityStatus::NotAvailable.as_str(), "not_available");
        assert_eq!(
            SPEC_CURSOR.capabilities.tokens,
            CapabilityStatus::NotAvailable
        );
        assert_eq!(SPEC_CODEX.capabilities.cost, CapabilityStatus::NotAvailable);
    }

    #[test]
    fn path_expectations_use_agent_home_not_trace_home() {
        let context = ImportContext::with_home("/tmp/trace-connection-test-home");
        assert_eq!(
            expected_path(&context, &SPEC_CLAUDE),
            PathBuf::from("/tmp/trace-connection-test-home/.claude/projects")
        );
    }
}
