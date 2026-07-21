//! Locating and loading the current Trace project.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use trace_core::{paths, ProjectConfig};

/// Walk up from `start` looking for a directory containing `.trace/config.toml`.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".trace").join("config.toml").is_file() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// A located project: its root directory and parsed config.
pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,
}

/// Load the project containing the current working directory, or error with a
/// hint to run `trace init`.
pub fn load_current() -> Result<Project> {
    let cwd = std::env::current_dir()?;
    let root = find_project_root(&cwd)
        .ok_or_else(|| anyhow!("not a Trace project (run `trace init` first)"))?;
    let config = ProjectConfig::load(&paths::project_config_path(&root))?;
    Ok(Project { root, config })
}
