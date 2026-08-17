//! Shared server state and the on-disk daemon state file (`~/.trace/daemon.json`).

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use trace_core::{paths, Store};

/// Application state shared across all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// The SQLite store. Wrapped in a mutex because `rusqlite::Connection` is
    /// not `Sync`; critical sections are short and never hold across `.await`.
    ///
    /// CONTENTION NOTE: this is a plain `Mutex`, so *every* request — reads
    /// included — serializes on it; concurrent GETs can't proceed in parallel.
    /// An `RwLock<Store>` (read guards for GET handlers, write guards for
    /// mutations) would let reads run concurrently. It was deliberately not
    /// done here: the store handle is also cloned into `cloud_sync::enqueue`
    /// (`Arc<Mutex<Store>>`) and constructed in `server.rs`, so the migration
    /// would have to change those modules too — out of scope for this pass. In
    /// practice contention is mild: the daemon binds to 127.0.0.1 with a small
    /// number of local clients and each critical section is a short SQLite call.
    /// Revisit as one atomic change (state + api + server + cloud_sync) if local
    /// read throughput ever becomes a bottleneck.
    pub store: Arc<Mutex<Store>>,
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
