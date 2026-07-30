//! `trg config show` and `trg config set <key> <value>` for the current project.

use anyhow::{anyhow, Result};
use traceguard_core::paths;

use crate::project;

/// Print the current project's config TOML.
pub fn show() -> Result<()> {
    let p = project::load_current()?;
    println!("{}", p.config.to_toml()?);
    Ok(())
}

/// Set a config key. Supports dotted keys for the prompt_compression table.
pub fn set(key: &str, value: &str) -> Result<()> {
    let p = project::load_current()?;
    let mut cfg = p.config;

    let parse_bool = |v: &str| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on");

    match key {
        "project_name" => cfg.project_name = value.to_string(),
        "prompt_compression.enabled" => cfg.prompt_compression.enabled = parse_bool(value),
        "prompt_compression.default_mode" => {
            if !["normal", "concise", "bare"].contains(&value) {
                return Err(anyhow!("default_mode must be normal|concise|bare"));
            }
            cfg.prompt_compression.default_mode = value.to_string();
        }
        "prompt_compression.prompt_history" => {
            cfg.prompt_compression.prompt_history = parse_bool(value)
        }
        "prompt_compression.external_llm" => {
            cfg.prompt_compression.external_llm = parse_bool(value)
        }
        "prompt_compression.default_output_budget" => {
            if !["tiny", "short", "normal", "detailed"].contains(&value) {
                return Err(anyhow!(
                    "default_output_budget must be tiny|short|normal|detailed"
                ));
            }
            cfg.prompt_compression.default_output_budget = value.to_string();
        }
        other => {
            return Err(anyhow!(
                "unknown or read-only key: {other}\nSettable keys: project_name, \
                 prompt_compression.{{enabled,default_mode,prompt_history,external_llm,default_output_budget}}"
            ));
        }
    }

    let path = paths::project_config_path(&p.root);
    cfg.save(&path)?;
    println!("Set {key} = {value}");
    Ok(())
}
