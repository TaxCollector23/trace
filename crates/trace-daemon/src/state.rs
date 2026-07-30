//! Shared server state and the on-disk daemon state file (`~/.trace/daemon.json`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use trace_core::{paths, GlobalConfig, Store};

/// Application state shared across all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// The SQLite store. Wrapped in a mutex because `rusqlite::Connection` is
    /// not `Sync`; critical sections are short and never hold across `.await`.
    pub store: Arc<Mutex<Store>>,
    /// Judge/key settings, loaded once at startup and mutated through the
    /// `/api/config/judge` route. Cheap to lock — read on nearly every judge
    /// call, written only when the user changes settings in the dashboard.
    pub global_config: Arc<Mutex<GlobalConfig>>,
    /// Last time the judge panel actually ran, per run id. Enforces a
    /// cooldown in `hook_check` (see `api.rs`) so a rapid save-loop —
    /// autosave, a formatter rewriting a file repeatedly, an editor's
    /// "format on every keystroke" mode — can't fire three paid model calls
    /// per save. Deliberately in-memory and per-daemon-lifetime: this is a
    /// spend guardrail, not an audit record, so it doesn't need to survive
    /// a restart or show up in the database.
    pub judge_cooldown: Arc<Mutex<HashMap<String, Instant>>>,
    pub port: u16,
    pub started_at: String,
    pub db_path: String,
}

/// Contents of `~/.trace/daemon.json`.
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
