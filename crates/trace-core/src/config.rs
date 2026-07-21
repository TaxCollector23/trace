//! Per-project configuration stored at `<project>/.trace/config.toml`.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
