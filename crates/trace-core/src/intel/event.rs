//! The normalized event model.
//!
//! This is the wire shape the v4 dashboard's `NormalizedEvent` TypeScript
//! interface (`apps/web/src/data.ts`) expects from `GET /api/runs/:id/events`.
//! It is deliberately a *superset-tolerant, evidence-only* shape: any field
//! the current storage layer cannot supply for a given row is left `None`
//! (or the explicit `Unknown`/`None` taxonomy member) rather than guessed.

use serde::{Deserialize, Serialize};

/// Lifecycle status of a normalized event. Mirrors the `EventStatus` union in
/// `apps/web/src/data.ts`. `Unknown` is the honest answer for an event kind
/// the mapper has no rule for yet — never silently defaulted to `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Started,
    Running,
    Ok,
    Warn,
    Blocked,
    Failed,
    PendingApproval,
    Unknown,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Started => "started",
            EventStatus::Running => "running",
            EventStatus::Ok => "ok",
            EventStatus::Warn => "warn",
            EventStatus::Blocked => "blocked",
            EventStatus::Failed => "failed",
            EventStatus::PendingApproval => "pending_approval",
            EventStatus::Unknown => "unknown",
        }
    }
}

/// Assessed risk level of a normalized event. Mirrors the `RiskLevel` union in
/// `apps/web/src/data.ts`. There is deliberately no `Unknown` variant here —
/// the TS contract's set is closed at `none..critical` — so an event kind the
/// mapper cannot characterize is given `None` (no *evidence* of risk), which
/// is the honest default, not a claim that it was checked and found safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::None => "none",
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}

/// A single normalized, source-agnostic event in a run's timeline.
///
/// Field-by-field provenance (see `mapper::normalize_run`):
/// - `id`, `run_id`, `ts_start`, `kind` — copied directly from the stored row.
/// - `parent_id`, `ts_end` — the current schema has no parent-event linkage or
///   explicit end timestamps, so these are always `None`. Never fabricated.
/// - `actor` — `runs.agent_name` when known, else the literal `"unknown"`
///   (an honest statement of absence, not a guessed name).
/// - `source` — always `"trace"`: every row in `events`/`commands` today was
///   recorded by Trace's own instrumentation; there is no multi-source
///   ingestion yet.
/// - `status`, `risk` — deterministically derived from the stored event/command
///   kind via a fixed lookup table (`mapper::status_and_risk_for_event_kind`).
///   An event kind with no table entry maps to `status: unknown`, `risk: none`.
/// - `target` — best-effort extraction from `metadata_json` (a `path`,
///   `file_path`, `git_ref`, or `command` field) when present; else `None`.
/// - `evidence` — the raw `message` plus parsed `metadata_json`, so the UI can
///   always show *something concrete* backing the row.
/// - `metadata` — `metadata_json` parsed as a JSON object, when it is one;
///   `None` otherwise (including parse failure — never a fabricated object).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub id: String,
    pub run_id: String,
    pub parent_id: Option<String>,
    pub ts_start: String,
    pub ts_end: Option<String>,
    pub kind: String,
    pub actor: String,
    pub source: String,
    pub status: String,
    pub risk: String,
    pub target: Option<String>,
    pub evidence: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}
