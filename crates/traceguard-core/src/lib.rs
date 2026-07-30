//! traceguard-core: shared models, rules, persistence, and git logic.
//!
//! This crate is the single source of truth for behaviour shared between the
//! `trg` CLI and the local daemon. It contains no I/O server code and no UI.

/// The user-facing product version. Single source of truth for the CLI,
/// daemon, dashboard, and docs. Bump this (and the workspace Cargo version)
/// when the project owner advances to the next subversion.
pub const VERSION: &str = "1.1";

/// The exact string printed by `trg --version` / `traceguard --version`.
pub fn version_string() -> String {
    format!("TraceGuard {VERSION}")
}

pub mod agents;
pub mod config;
pub mod cost;
pub mod db;
pub mod diagnose;
pub mod git;
pub mod github;
pub mod guard;
pub mod harden;
pub mod ids;
pub mod models;
pub mod paths;
pub mod prompt;
pub mod scan;
pub mod secrets;
pub mod time;

pub use config::ProjectConfig;
pub use db::Store;
pub use guard::{classify, Decision, GuardResult};
pub use models::*;
