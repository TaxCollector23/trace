//! Core data models and event types shared across the CLI, daemon, and core.
//!
//! These types are the wire format for the local API and the row shape for the
//! SQLite tables. Keep them serializable and free of behaviour.

use serde::{Deserialize, Serialize};

/// Lifecycle status of a monitored run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Blocked,
    RolledBack,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Blocked => "blocked",
            RunStatus::RolledBack => "rolled_back",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> RunStatus {
        match s {
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            "blocked" => RunStatus::Blocked,
            "rolled_back" => RunStatus::RolledBack,
            _ => RunStatus::Running,
        }
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

/// A stored TraceCompress record. Prompt text columns are only populated when
/// prompt history is enabled in the project config; otherwise only token
/// estimates and metadata are kept. Nothing is ever uploaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCompression {
    pub id: String,
    pub run_id: Option<String>,
    pub project_id: Option<String>,
    pub mode: String,
    pub original_token_estimate: i64,
    pub compressed_token_estimate: i64,
    pub estimated_reduction_percent: f64,
    pub compressed_prompt_hash: String,
    pub original_prompt_stored: bool,
    pub compressed_prompt_stored: bool,
    pub preserved_constraints_json: Option<String>,
    pub removed_redundancy_json: Option<String>,
    pub original_prompt: Option<String>,
    pub compressed_prompt: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPromptCompression {
    pub run_id: Option<String>,
    pub project_id: Option<String>,
    pub mode: String,
    pub original_token_estimate: i64,
    pub compressed_token_estimate: i64,
    pub estimated_reduction_percent: f64,
    pub compressed_prompt_hash: String,
    pub original_prompt_stored: bool,
    pub compressed_prompt_stored: bool,
    pub preserved_constraints_json: Option<String>,
    pub removed_redundancy_json: Option<String>,
    pub original_prompt: Option<String>,
    pub compressed_prompt: Option<String>,
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
}
