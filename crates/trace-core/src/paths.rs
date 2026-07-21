//! Global and per-project filesystem locations used by Trace.
//!
//! All global state lives under `~/.trace`. Per-project state lives under
//! `<project>/.trace`. Everything is local to the machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Root directory for all global Trace data: `~/.trace`.
pub fn global_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".trace"))
}

/// Path to the global SQLite database: `~/.trace/trace.db`.
pub fn database_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("trace.db"))
}

/// Path to the daemon state file: `~/.trace/daemon.json`.
pub fn daemon_state_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("daemon.json"))
}

/// Directory where install scripts place the `trace` binary.
pub fn bin_dir() -> Result<PathBuf> {
    Ok(global_dir()?.join("bin"))
}

/// Ensure the global directory exists, creating it if necessary.
pub fn ensure_global_dir() -> Result<PathBuf> {
    let dir = global_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating global dir {}", dir.display()))?;
    Ok(dir)
}

/// The per-project `.trace` directory for the given project root.
pub fn project_dir(project_root: &Path) -> PathBuf {
    project_root.join(".trace")
}

/// The per-project config file path.
pub fn project_config_path(project_root: &Path) -> PathBuf {
    project_dir(project_root).join("config.toml")
}

/// Directory holding captured logs for a run: `<project>/.trace/runs/<run_id>`.
pub fn run_log_dir(project_root: &Path, run_id: &str) -> PathBuf {
    project_dir(project_root).join("runs").join(run_id)
}
