//! Shared server state and the on-disk daemon state file (`~/.trace/daemon.json`).

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use trace_core::{paths, Store};

/// Application state shared across all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// The SQLite store. Wrapped in a `Mutex` because `Store` holds a bare
    /// `rusqlite::Connection`, which is `Send` but deliberately **`!Sync`** (its
    /// `&self` methods use interior mutability). Critical sections are short and
    /// never held across `.await`.
    ///
    /// This is NOT swappable for `RwLock<Store>`: `RwLock<T>: Sync` requires
    /// `T: Sync`, and handing out `&Store` to multiple reader threads at once
    /// is exactly what makes a single SQLite `Connection` unsound — Rust rejects
    /// it (`AppState` would stop being `Send + Sync` and every axum handler
    /// fails to compile). The `Mutex` is forced by rusqlite's threading model,
    /// not an arbitrary choice. True concurrent reads would require making
    /// `Store` itself `Sync` — a connection-pool refactor in `trace-core`
    /// (e.g. r2d2/deadpool, one connection per reader) — which is a much larger
    /// change than a lock swap. In practice contention is mild anyway: the
    /// daemon binds to 127.0.0.1 for a few local clients and each critical
    /// section is a short SQLite call.
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
