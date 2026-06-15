//! Shared server state and the on-disk daemon state file (`~/.traceguard/daemon.json`).

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use traceguard_core::{paths, Store};

/// Application state shared across all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// The SQLite store. Wrapped in a mutex because `rusqlite::Connection` is
    /// not `Sync`; critical sections are short and never hold across `.await`.
    pub store: Arc<Mutex<Store>>,
    pub port: u16,
    pub started_at: String,
    pub db_path: String,
}

/// Contents of `~/.traceguard/daemon.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonState {
    pub pid: u32,
    pub port: u16,
    pub started_at: String,
}

impl DaemonState {
    /// Persist the daemon state file.
    pub fn write(&self) -> Result<()> {
        paths::ensure_global_dir()?;
        let path = paths::daemon_state_path()?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Read the daemon state file if present.
    pub fn read() -> Result<Option<DaemonState>> {
        let path = paths::daemon_state_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw).ok())
    }

    /// Remove the daemon state file (on shutdown).
    pub fn clear() -> Result<()> {
        let path = paths::daemon_state_path()?;
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
