//! Per-project configuration stored at `<project>/.traceguard/config.toml`.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// TraceCompress (prompt compression) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCompressionConfig {
    /// Master switch for the compression feature.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default mode: "normal" | "concise" | "bare".
    #[serde(default = "default_mode")]
    pub default_mode: String,
    /// Store prompt text locally for history. Token estimates and metadata are
    /// always stored; the raw prompt text is only stored when this is true.
    /// Never uploaded regardless.
    #[serde(default = "default_true")]
    pub prompt_history: bool,
    /// Allow opt-in external-LLM compression (off by default; would send prompt
    /// text off the machine, so it must be explicit).
    #[serde(default)]
    pub external_llm: bool,
    /// Default output-budget preset: tiny | short | normal | detailed.
    #[serde(default = "default_output_budget")]
    pub default_output_budget: String,
}

fn default_output_budget() -> String {
    "short".to_string()
}

fn default_true() -> bool {
    true
}

fn default_mode() -> String {
    "concise".to_string()
}

impl Default for PromptCompressionConfig {
    fn default() -> Self {
        PromptCompressionConfig {
            enabled: true,
            default_mode: default_mode(),
            prompt_history: true,
            external_llm: false,
            default_output_budget: default_output_budget(),
        }
    }
}

/// Project configuration. Kept intentionally small for the MVP.
///
/// Note: table fields (`prompt_compression`) must be serialized after all
/// scalar/array fields, so it is declared last.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project_name: String,
    #[serde(default = "default_protected_files")]
    pub protected_files: Vec<String>,
    #[serde(default = "default_checks")]
    pub default_checks: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub prompt_compression: PromptCompressionConfig,
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
            prompt_compression: PromptCompressionConfig::default(),
        }
    }

    /// Serialize to a TOML string with a friendly header comment.
    pub fn to_toml(&self) -> Result<String> {
        let body = toml::to_string_pretty(self).context("serializing project config")?;
        Ok(format!(
            "# TraceGuard project configuration\n# Docs: https://github.com/TaxCollector23/TraceGuard\n\n{body}"
        ))
    }

    /// Load config from a `.traceguard/config.toml` path.
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
