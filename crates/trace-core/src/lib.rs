//! trace-core: shared models, rules, persistence, and git logic.
//!
//! This crate is the single source of truth for behaviour shared between the
//! `trace` CLI and the local daemon. It contains no I/O server code and no UI.

/// The user-facing product version. Single source of truth for the CLI,
/// daemon, dashboard, and docs. Sourced directly from the workspace Cargo
/// version (`CARGO_PKG_VERSION`) so it can never drift from `Cargo.toml`
/// again — previously this was a hardcoded `"1.3"` that lagged the real
/// `1.3.3` (see RECOVERY-AUDIT versioning note).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The exact first line printed by `trc --version` / shown by `trc doctor`.
pub fn version_string() -> String {
    format!("Trace {VERSION}")
}

/// The active rule pack's version string (e.g. `2025.08.4`).
pub fn rule_pack_version() -> String {
    rules_pack::active().version.clone()
}

/// Total number of rules in the active pack: command rules + policy rules +
/// secret patterns + injection phrases. A single honest count for
/// `--version`/`doctor` (which previously exposed neither pack version nor
/// rule count).
pub fn rule_count() -> usize {
    rules_pack::active().rule_count()
}

/// A one-line version summary showing Trace version, rule-pack version, and
/// rule count together — the single string `--version` and `doctor` share.
pub fn version_details() -> String {
    format!(
        "Trace {VERSION} · rule pack {} ({} rules)",
        rule_pack_version(),
        rule_count()
    )
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
pub mod integrations;
pub mod intel;
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

#[cfg(test)]
mod version_tests {
    #[test]
    fn version_tracks_the_cargo_workspace_version() {
        // Guards the old hardcode drift: VERSION must equal the crate version,
        // which inherits the workspace `version` in Cargo.toml.
        assert_eq!(super::VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn version_details_shows_trace_pack_and_rule_count() {
        let d = super::version_details();
        assert!(d.starts_with("Trace "));
        assert!(d.contains("rule pack"));
        assert!(d.contains("rules"));
        assert_eq!(
            super::rule_count(),
            super::rules_pack::active().rule_count()
        );
    }
}
