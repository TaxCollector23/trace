//! trace-core: shared models, rules, persistence, and git logic.
//!
//! This crate is the single source of truth for behaviour shared between the
//! `trace` CLI and the local daemon. It contains no I/O server code and no UI.

/// The user-facing product version. Single source of truth for the CLI,
/// daemon, dashboard, and docs. Bump this (and the workspace Cargo version)
/// when the project owner advances to the next subversion.
pub const VERSION: &str = "1.2";

/// The exact string printed by `trc --version`.
pub fn version_string() -> String {
    format!("Trace {VERSION}")
}

pub mod adapter;
pub mod agents;
pub mod compress;
pub mod config;
pub mod cost;
pub mod db;
pub mod diagnose;
pub mod eval;
pub mod git;
pub mod github;
pub mod guard;
pub mod ids;
pub mod models;
pub mod paths;
pub mod policy;
pub mod prompt_quality;
pub mod ratify;
pub mod redteam;
pub mod rules_pack;
pub mod scan;
pub mod secrets;
pub mod time;

pub use compress::{decode as decompress_stored, encode as compress_for_storage, CompressionStats};
pub use config::ProjectConfig;
pub use db::Store;
pub use eval::{run_policy_eval, PolicyEvalReport};
pub use guard::{classify, Decision, GuardResult};
pub use models::*;
pub use policy::{run_policy_checks, FileDiff, PolicyFinding, Severity};
pub use prompt_quality::{analyze_prompt, PromptAnalysis, PromptPattern};
pub use ratify::{summarize as ratify_summarize, RatifySummary, RatifyVerdict, SeverityCounts};
pub use redteam::{run_redteam_eval, EngineScore, RedTeamReport};
