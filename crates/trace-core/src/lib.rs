//! trace-core: shared models, rules, persistence, and git logic.
//!
//! This crate is the single source of truth for behaviour shared between the
//! `trace` CLI and the local daemon. It contains no I/O server code and no UI.

/// The user-facing product version. Single source of truth for the CLI,
/// daemon, dashboard, and docs. Bump this (and the workspace Cargo version)
/// when the project owner advances to the next subversion.
pub const VERSION: &str = "1.2";

/// The exact string printed by `trace --version`.
pub fn version_string() -> String {
    format!("Trace {VERSION}")
}

pub mod adapter;
pub mod agents;
pub mod coaching;
pub mod config;
pub mod cost;
pub mod db;
pub mod diagnose;
pub mod doctrine;
pub mod eval;
pub mod git;
pub mod github;
pub mod guard;
pub mod ids;
pub mod judge;
pub mod models;
pub mod paths;
pub mod policy;
pub mod prompt_quality;
pub mod scan;
pub mod secrets;
pub mod time;

pub use coaching::{build_report as build_coaching_report, CoachingReport, PatternStat};
pub use config::{GlobalConfig, ProjectConfig};
pub use db::Store;
pub use doctrine::{mine_doctrine, MinedRule, MiningResult, RuleStrength};
pub use eval::{run_policy_eval, PolicyEvalReport};
pub use guard::{classify, Decision, GuardResult};
pub use judge::{run_judge, JudgeContext, JudgeMode, JudgeSettings, JudgeVerdict, JudgeVote};
pub use models::*;
pub use policy::{run_policy_checks, FileDiff, PolicyFinding, Severity};
pub use prompt_quality::{analyze_prompt, PromptAnalysis, PromptPattern};
