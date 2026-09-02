//! Core data models and event types shared across the CLI, daemon, and core.
//!
//! These types are the wire format for the local API and the row shape for the
//! SQLite tables. Keep them serializable and free of behaviour.

use serde::{Deserialize, Serialize};

/// Lifecycle status of a monitored run.
///
/// This is the *persistence* status stored in `runs.status`. It is a strict,
/// closed set: an unrecognized or truncated status is decoded to
/// [`RunStatus::Unknown`], **never** silently coerced to `Running` (doing so
/// was the root cause of "stuck forever" runs — see RECOVERY-AUDIT fix #1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run's wrapper process is alive and actively monitored.
    Running,
    Completed,
    Failed,
    Blocked,
    RolledBack,
    /// The user asked to stop the run before it finished (explicit cancel).
    Cancelled,
    /// The run was aborted by Trace or a guard/policy before completing.
    Aborted,
    /// The wrapper died without a clean finish (Ctrl-C, crash, host reboot).
    /// Assigned by zombie reconciliation on daemon startup, never guessed.
    Interrupted,
    /// The stored status string was empty or unrecognized. Modeled explicitly
    /// so a decode failure is visible rather than masquerading as `running`.
    Unknown,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Blocked => "blocked",
            RunStatus::RolledBack => "rolled_back",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Aborted => "aborted",
            RunStatus::Interrupted => "interrupted",
            RunStatus::Unknown => "unknown",
        }
    }

    /// True once the run has reached a terminal state (will never move again on
    /// its own). `Running` is the only non-terminal state.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, RunStatus::Running)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> RunStatus {
        match s {
            "running" => RunStatus::Running,
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            "blocked" => RunStatus::Blocked,
            "rolled_back" => RunStatus::RolledBack,
            "cancelled" => RunStatus::Cancelled,
            "aborted" => RunStatus::Aborted,
            "interrupted" => RunStatus::Interrupted,
            // Never coerce an unknown status back to `running`: a run we cannot
            // decode is surfaced as `Unknown`, not silently resurrected.
            _ => RunStatus::Unknown,
        }
    }
}

/// Richer *terminal outcome* of a run, derived from structured evidence at
/// finish time (exit code, guard/policy decisions, checkpoint state). This is
/// distinct from [`RunStatus`], which tracks lifecycle. The outcome answers
/// "how did it actually go?" without inventing semantic correctness: a run that
/// exits 0 but tripped a policy block is `Blocked`, not `Success`.
///
/// Per RECOVERY-AUDIT fix #1 / prompt §36. Deterministic and local — no LLM,
/// no guessing. When evidence is genuinely absent the outcome is [`Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunOutcome {
    Success,
    SuccessWithWarnings,
    Partial,
    Failed,
    Blocked,
    Cancelled,
    RolledBack,
    Unknown,
}

impl RunOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunOutcome::Success => "SUCCESS",
            RunOutcome::SuccessWithWarnings => "SUCCESS_WITH_WARNINGS",
            RunOutcome::Partial => "PARTIAL",
            RunOutcome::Failed => "FAILED",
            RunOutcome::Blocked => "BLOCKED",
            RunOutcome::Cancelled => "CANCELLED",
            RunOutcome::RolledBack => "ROLLED_BACK",
            RunOutcome::Unknown => "UNKNOWN",
        }
    }

    /// Deterministically derive an outcome from the run's structured evidence.
    ///
    /// Inputs are facts Trace already records, never inferred semantics:
    /// - `status`: the persisted lifecycle status.
    /// - `exit_code`: the wrapped command's exit code (`None` if it never
    ///   produced one — e.g. an interrupted run).
    /// - `had_block`: a guard/policy `block` decision fired during the run.
    /// - `had_warning`: a warn/require-approval decision or secret warning fired.
    ///
    /// The key invariant (prompt §36): an exit-0 run that hit a block/warning
    /// must NOT look perfectly successful.
    pub fn derive(
        status: RunStatus,
        exit_code: Option<i64>,
        had_block: bool,
        had_warning: bool,
    ) -> RunOutcome {
        match status {
            RunStatus::Running => RunOutcome::Unknown,
            RunStatus::RolledBack => RunOutcome::RolledBack,
            RunStatus::Blocked => RunOutcome::Blocked,
            RunStatus::Cancelled => RunOutcome::Cancelled,
            // An interrupted/aborted run left work in an indeterminate state.
            RunStatus::Interrupted | RunStatus::Aborted => RunOutcome::Partial,
            RunStatus::Unknown => RunOutcome::Unknown,
            RunStatus::Failed => RunOutcome::Failed,
            RunStatus::Completed => {
                if had_block {
                    // Exited "completed" yet a hard block fired: not a success.
                    RunOutcome::Blocked
                } else if exit_code.is_some_and(|c| c != 0) {
                    RunOutcome::Failed
                } else if had_warning {
                    RunOutcome::SuccessWithWarnings
                } else {
                    RunOutcome::Success
                }
            }
        }
    }
}

/// Row counts for the telemetry tables purged by `trc reset --local-data`.
/// Shown to the user before deletion so the confirmation is exact, not vague.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryCounts {
    pub runs: i64,
    pub commands: i64,
    pub events: i64,
    pub checkpoints: i64,
}

impl TelemetryCounts {
    /// True when there is nothing to delete.
    pub fn is_empty(&self) -> bool {
        self.runs == 0 && self.commands == 0 && self.events == 0 && self.checkpoints == 0
    }
}

/// A registered project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub config_path: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A single monitored session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub project_id: String,
    pub command: String,
    pub agent_name: Option<String>,
    pub user_prompt: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub starting_commit: Option<String>,
    pub ending_commit: Option<String>,
    pub status: String,
    pub exit_code: Option<i64>,
    pub created_at: String,
}

/// Request body to create a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRun {
    pub project_id: String,
    pub command: String,
    pub agent_name: Option<String>,
    pub user_prompt: Option<String>,
    pub starting_commit: Option<String>,
}

/// Request body to register/upsert a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub path: String,
    pub config_path: String,
}

/// Categories of timeline events. Stored as `type` on the events table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    RunCreated,
    CheckpointCreated,
    CommandStarted,
    FileCreated,
    FileModified,
    FileDeleted,
    RiskyCommandWarned,
    RiskyCommandBlocked,
    CommandApproved,
    SecretWarning,
    ApiUsageRecorded,
    BuildStarted,
    BuildFailed,
    BuildPassed,
    TestsPassed,
    TestsFailed,
    FinalDiffCaptured,
    RunCompleted,
    RunFailed,
    RollbackCreated,
    RollbackCompleted,
    Note,

    // --- Unified adapter event vocabulary (see core::adapter) ---
    // Every adapter (Claude today; Cursor/Codex/etc. later) emits from this
    // same set, so the replay engine and dashboard never special-case an
    // agent. Some overlap with the events above is intentional: those
    // predate the adapter system and are kept so existing runs/dashboards
    // don't need a migration.
    SessionStarted,
    SessionEnded,
    PromptSubmitted,
    PromptFinished,
    ToolCallStarted,
    ToolCallFinished,
    CommandOutput,
    FileOpened,
    DirectoryCreated,
    DirectoryDeleted,
    GitStatusChanged,
    CommitDetected,
    BranchChanged,
    TestsStarted,
    AgentIdle,
    AgentThinking,
    ReplayMarker,
    RiskDetected,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::RunCreated => "run_created",
            EventType::CheckpointCreated => "checkpoint_created",
            EventType::CommandStarted => "command_started",
            EventType::FileCreated => "file_created",
            EventType::FileModified => "file_modified",
            EventType::FileDeleted => "file_deleted",
            EventType::RiskyCommandWarned => "risky_command_warned",
            EventType::RiskyCommandBlocked => "risky_command_blocked",
            EventType::CommandApproved => "command_approved",
            EventType::SecretWarning => "secret_warning",
            EventType::ApiUsageRecorded => "api_usage_recorded",
            EventType::BuildStarted => "build_started",
            EventType::BuildFailed => "build_failed",
            EventType::BuildPassed => "build_passed",
            EventType::TestsPassed => "tests_passed",
            EventType::TestsFailed => "tests_failed",
            EventType::FinalDiffCaptured => "final_diff_captured",
            EventType::RunCompleted => "run_completed",
            EventType::RunFailed => "run_failed",
            EventType::RollbackCreated => "rollback_created",
            EventType::RollbackCompleted => "rollback_completed",
            EventType::Note => "note",
            EventType::SessionStarted => "session_started",
            EventType::SessionEnded => "session_ended",
            EventType::PromptSubmitted => "prompt_submitted",
            EventType::PromptFinished => "prompt_finished",
            EventType::ToolCallStarted => "tool_call_started",
            EventType::ToolCallFinished => "tool_call_finished",
            EventType::CommandOutput => "command_output",
            EventType::FileOpened => "file_opened",
            EventType::DirectoryCreated => "directory_created",
            EventType::DirectoryDeleted => "directory_deleted",
            EventType::GitStatusChanged => "git_status_changed",
            EventType::CommitDetected => "commit_detected",
            EventType::BranchChanged => "branch_changed",
            EventType::TestsStarted => "tests_started",
            EventType::AgentIdle => "agent_idle",
            EventType::AgentThinking => "agent_thinking",
            EventType::ReplayMarker => "replay_marker",
            EventType::RiskDetected => "risk_detected",
        }
    }
}

/// A timeline event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub run_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: String,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

/// Body for appending an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: String,
    pub metadata_json: Option<String>,
}

/// How a file changed, derived from the final git diff (source of truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::Created => "created",
            ChangeType::Modified => "modified",
            ChangeType::Deleted => "deleted",
            ChangeType::Renamed => "renamed",
        }
    }

    /// The vocabulary `policy.rs`'s checks actually match against — the same
    /// one GitHub's own PR-files API uses ("added"/"removed", not
    /// "created"/"deleted"). `as_str()` above is for display/storage; this
    /// is for anything that feeds `policy::FileDiff.status`. Keeping these
    /// separate rather than changing `as_str()` avoids a silent behavior
    /// change everywhere `as_str()` is already used for display/persistence.
    pub fn as_diff_status(&self) -> &'static str {
        match self {
            ChangeType::Created => "added",
            ChangeType::Modified => "modified",
            ChangeType::Deleted => "removed",
            ChangeType::Renamed => "renamed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub id: String,
    pub run_id: String,
    pub path: String,
    pub change_type: String,
    pub diff_summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFileChange {
    pub path: String,
    pub change_type: String,
    pub diff_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    pub id: String,
    pub run_id: String,
    pub command: String,
    pub decision: String,
    pub exit_code: Option<i64>,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCommand {
    pub command: String,
    pub decision: String,
    pub exit_code: Option<i64>,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRecord {
    pub id: String,
    pub run_id: String,
    pub file_path: Option<String>,
    pub secret_type: String,
    pub redacted_value: String,
    pub action_taken: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSecret {
    pub file_path: Option<String>,
    pub secret_type: String,
    pub redacted_value: String,
    pub action_taken: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiUsage {
    pub id: String,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cached_tokens: Option<i64>,
    pub estimated_cost: Option<f64>,
    pub latency_ms: Option<i64>,
    pub created_at: String,
}

/// Token/run totals for one agent, across every run that ever recorded
/// API usage — the "how much have I used Claude Code vs Codex" breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTokenStats {
    pub agent_name: String,
    pub run_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub estimated_cost: f64,
}

/// Usage analytics across every run ever recorded, computed from real data
/// (no telemetry, no network — this is a local SQLite aggregate query).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_runs: i64,
    pub first_run_at: Option<String>,
    /// Average runs per hour/day/week/month since the first recorded run.
    /// `None` when there isn't enough history yet (fewer than 2 runs, or
    /// the first run was less than an hour ago).
    pub avg_per_hour: Option<f64>,
    pub avg_per_day: Option<f64>,
    pub avg_per_week: Option<f64>,
    pub avg_per_month: Option<f64>,
    pub by_agent: Vec<AgentTokenStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApiUsage {
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cached_tokens: Option<i64>,
    pub estimated_cost: Option<f64>,
    pub latency_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub run_id: String,
    pub project_id: String,
    pub git_ref: Option<String>,
    pub checkpoint_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCheckpoint {
    pub project_id: String,
    pub git_ref: Option<String>,
    pub checkpoint_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub run_id: String,
    pub command: String,
    pub status: String,
    pub output_summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTestResult {
    pub command: String,
    pub status: String,
    pub output_summary: Option<String>,
}

/// Aggregated counts shown on a run card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    #[serde(flatten)]
    pub run: Run,
    pub project_name: String,
    pub files_changed: i64,
    pub command_count: i64,
    pub secret_warnings: i64,
    pub estimated_cost: Option<f64>,
    pub checks_status: Option<String>,
    /// Deterministically derived terminal outcome (see [`RunOutcome`]), as its
    /// `SCREAMING_SNAKE_CASE` string. `UNKNOWN` while the run is still running
    /// or when evidence is genuinely absent — never a guess.
    pub outcome: String,
}

// --- Policy findings --------------------------------------------------------
// Storage-layer records for the deterministic policy engine (policy.rs). Kept
// as plain-string severity fields here (rather than the typed enums used in
// policy.rs/guard.rs) since these are wire/row shapes read straight off SQLite
// and serialized straight to the dashboard — the typed enums are the source of
// truth when a finding is first produced.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFindingRecord {
    pub id: String,
    pub run_id: String,
    pub rule_key: String,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub severity: String,
    pub confidence: f64,
    pub source: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_never_coerces_unknown_to_running() {
        // The regression this guards: a truncated/garbage status must decode to
        // Unknown, NOT Running (which made crashed runs look alive forever).
        assert_eq!(RunStatus::from_str("running"), RunStatus::Running);
        assert_eq!(RunStatus::from_str("interrupted"), RunStatus::Interrupted);
        assert_eq!(RunStatus::from_str("cancelled"), RunStatus::Cancelled);
        assert_eq!(RunStatus::from_str("aborted"), RunStatus::Aborted);
        assert_eq!(RunStatus::from_str(""), RunStatus::Unknown);
        assert_eq!(RunStatus::from_str("garbage"), RunStatus::Unknown);
        assert_ne!(RunStatus::from_str("garbage"), RunStatus::Running);
    }

    #[test]
    fn run_status_str_roundtrips() {
        for s in [
            RunStatus::Running,
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::Blocked,
            RunStatus::RolledBack,
            RunStatus::Cancelled,
            RunStatus::Aborted,
            RunStatus::Interrupted,
            RunStatus::Unknown,
        ] {
            assert_eq!(RunStatus::from_str(s.as_str()), s);
        }
        assert!(!RunStatus::Running.is_terminal());
        assert!(RunStatus::Interrupted.is_terminal());
    }

    #[test]
    fn outcome_exit_zero_with_block_is_not_success() {
        // The core §36 invariant: exit 0 but a policy/guard block fired must not
        // read as a clean success.
        let o = RunOutcome::derive(RunStatus::Completed, Some(0), true, false);
        assert_eq!(o, RunOutcome::Blocked);
        assert_ne!(o, RunOutcome::Success);
    }

    #[test]
    fn outcome_exit_zero_with_warning_is_success_with_warnings() {
        let o = RunOutcome::derive(RunStatus::Completed, Some(0), false, true);
        assert_eq!(o, RunOutcome::SuccessWithWarnings);
    }

    #[test]
    fn outcome_clean_exit_zero_is_success() {
        assert_eq!(
            RunOutcome::derive(RunStatus::Completed, Some(0), false, false),
            RunOutcome::Success
        );
    }

    #[test]
    fn outcome_nonzero_exit_is_failed_even_if_status_completed() {
        assert_eq!(
            RunOutcome::derive(RunStatus::Completed, Some(1), false, false),
            RunOutcome::Failed
        );
    }

    #[test]
    fn outcome_interrupted_is_partial_not_success() {
        let o = RunOutcome::derive(RunStatus::Interrupted, None, false, false);
        assert_eq!(o, RunOutcome::Partial);
    }

    #[test]
    fn outcome_running_and_unknown_are_unknown() {
        assert_eq!(
            RunOutcome::derive(RunStatus::Running, None, false, false),
            RunOutcome::Unknown
        );
        assert_eq!(
            RunOutcome::derive(RunStatus::Unknown, None, false, false),
            RunOutcome::Unknown
        );
    }
}
