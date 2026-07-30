//! Per-project configuration stored at `<project>/.trace/config.toml`, and
//! global (machine-wide) configuration stored at `~/.trace/global.toml`.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::judge::JudgeSettings;

/// Project configuration. Kept intentionally small for the MVP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project_name: String,
    #[serde(default = "default_protected_files")]
    pub protected_files: Vec<String>,
    #[serde(default = "default_checks")]
    pub default_checks: Vec<String>,
    #[serde(default)]
    pub created_at: String,
}

fn default_protected_files() -> Vec<String> {
    vec![
        ".env".into(),
        ".env.local".into(),
        "id_rsa".into(),
        "secrets.json".into(),
    ]
}

fn default_checks() -> Vec<String> {
    Vec::new()
}

impl ProjectConfig {
    /// Build a fresh config for a newly initialized project.
    pub fn new(project_name: impl Into<String>, created_at: impl Into<String>) -> Self {
        ProjectConfig {
            project_name: project_name.into(),
            protected_files: default_protected_files(),
            default_checks: default_checks(),
            created_at: created_at.into(),
        }
    }

    /// Serialize to a TOML string with a friendly header comment.
    pub fn to_toml(&self) -> Result<String> {
        let body = toml::to_string_pretty(self).context("serializing project config")?;
        Ok(format!(
            "# Trace project configuration\n# Docs: https://github.com/TaxCollector23/trace\n\n{body}"
        ))
    }

    /// Load config from a `.trace/config.toml` path.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let cfg: ProjectConfig =
            toml::from_str(&raw).with_context(|| format!("parsing config {}", path.display()))?;
        Ok(cfg)
    }

    /// Write config to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(path, self.to_toml()?)
            .with_context(|| format!("writing config {}", path.display()))?;
        Ok(())
    }

    /// Whether a path matches one of the protected file rules (by file name or suffix).
    pub fn is_protected(&self, path: &str) -> bool {
        let normalized = path.replace('\\', "/");
        let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
        self.protected_files.iter().any(|rule| {
            rule == file_name || normalized.ends_with(rule.as_str()) || rule == &normalized
        })
    }
}

/// Machine-wide configuration: `~/.trace/global.toml`. Holds settings that
/// apply across every project, most importantly the judge panel setup.
/// Provider API keys, when present, live only here (or in environment
/// variables that override it) — never in the binary, never in a
/// per-project file that might get committed to a repo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub judge: JudgeSettings,
}

/// Env var overrides, checked before the file value for each slot. Derived
/// generically from the provider id (`"deepseek"` -> `TRACE_DEEPSEEK_API_KEY`)
/// so a newly-added provider slot gets env var support for free — no code
/// change needed here when someone points a slot at a new lab.
fn env_key_for(provider: &str) -> Option<String> {
    let normalized = provider.trim();
    if normalized.is_empty() {
        return None;
    }
    let upper = normalized.to_uppercase().replace(['-', ' '], "_");
    // TRACE_-prefixed form wins so a user with e.g. a global OPENAI_API_KEY
    // for other tools can still point Trace at a different key. Then fall
    // back to the ecosystem-standard bare form (OPENROUTER_API_KEY,
    // ANTHROPIC_API_KEY, ...) so a fresh user with one key in .env.local
    // gets a working panel with zero extra config.
    for var in [format!("TRACE_{upper}_API_KEY"), format!("{upper}_API_KEY")] {
        if let Ok(v) = std::env::var(&var) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

impl GlobalConfig {
    /// Load from `~/.trace/global.toml`, applying environment variable
    /// overrides for provider keys. Returns the default (judge disabled)
    /// if no file exists yet — first run should never error.
    pub fn load() -> Result<Self> {
        let path = crate::paths::global_dir()?.join("global.toml");
        let mut cfg: GlobalConfig = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading global config {}", path.display()))?;
            toml::from_str(&raw).with_context(|| format!("parsing global config {}", path.display()))?
        } else {
            GlobalConfig::default()
        };

        for slot in &mut cfg.judge.slots {
            if let Some(env_key) = env_key_for(&slot.provider) {
                slot.api_key = Some(env_key);
            }
        }

        Ok(cfg)
    }

    /// Persist to `~/.trace/global.toml`. Keys typed in via env vars are
    /// never written back to disk by this path — only whatever the caller
    /// explicitly set on `judge.slots[].api_key` before calling `save`.
    pub fn save(&self) -> Result<()> {
        let dir = crate::paths::ensure_global_dir()?;
        let path = dir.join("global.toml");
        let body = toml::to_string_pretty(self).context("serializing global config")?;
        std::fs::write(&path, body).with_context(|| format!("writing global config {}", path.display()))?;
        Ok(())
    }

    /// A copy safe to send to the frontend: every API key replaced with a
    /// presence flag instead of the raw value.
    pub fn redacted(&self) -> GlobalConfig {
        let mut clone = self.clone();
        for slot in &mut clone.judge.slots {
            slot.api_key = slot.api_key.as_ref().map(|_| "••••••••".to_string());
        }
        clone
    }
}
