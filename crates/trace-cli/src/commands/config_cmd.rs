//! `trace config show` and `trace config set <key> <value>` for the current project.

use anyhow::{anyhow, Result};
use trace_core::paths;

use crate::colors;
use crate::project;

/// Print the current project's config TOML.
pub fn show() -> Result<()> {
    let p = project::load_current()?;
    println!("{}", p.config.to_toml()?);
    Ok(())
}

/// Set a config key.
pub fn set(key: &str, value: &str) -> Result<()> {
    let p = project::load_current()?;
    let mut cfg = p.config;

    match key {
        "project_name" => cfg.project_name = value.to_string(),
        other => {
            return Err(anyhow!(
                "unknown or read-only key: {other}\nSettable keys: project_name"
            ));
        }
    }

    let path = paths::project_config_path(&p.root);
    cfg.save(&path)?;
    println!("{} {key} = {value}", colors::green("Set"));
    Ok(())
}
