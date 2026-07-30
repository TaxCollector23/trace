//! Global TraceGuard CLI settings (`~/.traceguard/settings.json`).
//!
//! Small, local-only preferences shared across commands: the default launch
//! target and default hardening mode.

use serde::{Deserialize, Serialize};
use traceguard_core::paths;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Default `trg launch` target (agent id).
    pub default_target: Option<String>,
    /// Default `trg guard` mode (e.g. "balanced", "coding").
    pub default_mode: Option<String>,
}

fn path() -> Option<std::path::PathBuf> {
    paths::global_dir().ok().map(|d| d.join("settings.json"))
}

/// Load settings, returning defaults if the file is absent or invalid.
pub fn load() -> Settings {
    let Some(p) = path() else {
        return Settings::default();
    };
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist settings to disk.
pub fn save(s: &Settings) -> anyhow::Result<()> {
    paths::ensure_global_dir()?;
    let p = path().ok_or_else(|| anyhow::anyhow!("no home directory"))?;
    std::fs::write(p, serde_json::to_string_pretty(s)?)?;
    Ok(())
}
