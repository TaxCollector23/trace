//! Global and per-project filesystem locations used by TraceGuard.
//!
//! All TraceGuard global state lives under `~/.traceguard`. Per-project state
//! lives under `<project>/.traceguard`. Everything is local to the machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Root directory for all global TraceGuard data: `~/.traceguard`.
pub fn global_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".traceguard"))
}

/// Path to the global SQLite database: `~/.traceguard/traceguard.db`.
pub fn database_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("traceguard.db"))
}

/// Path to the daemon state file: `~/.traceguard/daemon.json`.
pub fn daemon_state_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("daemon.json"))
}

/// Directory where install scripts place the `trg` binary.
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

/// The per-project `.traceguard` directory for the given project root.
pub fn project_dir(project_root: &Path) -> PathBuf {
    project_root.join(".traceguard")
}

/// The per-project config file path.
pub fn project_config_path(project_root: &Path) -> PathBuf {
    project_dir(project_root).join("config.toml")
}

/// Directory holding captured logs for a run: `<project>/.traceguard/runs/<run_id>`.
pub fn run_log_dir(project_root: &Path, run_id: &str) -> PathBuf {
    project_dir(project_root).join("runs").join(run_id)
}
