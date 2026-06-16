//! traceguard-core: shared models, rules, persistence, and git logic.
//!
//! This crate is the single source of truth for behaviour shared between the
//! `trg` CLI and the local daemon. It contains no I/O server code and no UI.

pub mod config;
pub mod cost;
pub mod db;
pub mod diagnose;
pub mod git;
pub mod github;
pub mod guard;
pub mod ids;
pub mod models;
pub mod paths;
pub mod prompt;
pub mod secrets;
pub mod time;

pub use config::ProjectConfig;
pub use db::Store;
pub use guard::{classify, Decision, GuardResult};
pub use models::*;
