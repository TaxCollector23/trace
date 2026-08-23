//! Global and per-project filesystem locations used by Trace.
//!
//! All global state lives under `~/.trace`. Per-project state lives under
//! `<project>/.trace`. Everything is local to the machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Root directory for all global Trace data: `~/.trace`.
pub fn global_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".trace"))
}

/// Path to the global SQLite database: `~/.trace/trace.db`.
pub fn database_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("trace.db"))
}

/// Path to the daemon state file: `~/.trace/daemon.json`.
pub fn daemon_state_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("daemon.json"))
}

/// Directory where install scripts place the `trace` binary.
pub fn bin_dir() -> Result<PathBuf> {
    Ok(global_dir()?.join("bin"))
}

/// Ensure the global directory exists, creating it if necessary.
pub fn ensure_global_dir() -> Result<PathBuf> {
    let dir = global_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating global dir {}", dir.display()))?;
    Ok(dir)
}

/// The per-project `.trace` directory for the given project root.
pub fn project_dir(project_root: &Path) -> PathBuf {
    project_root.join(".trace")
}

/// The per-project config file path.
pub fn project_config_path(project_root: &Path) -> PathBuf {
    project_dir(project_root).join("config.toml")
}

/// Directory holding captured logs for a run: `<project>/.trace/runs/<run_id>`.
pub fn run_log_dir(project_root: &Path, run_id: &str) -> PathBuf {
    project_dir(project_root).join("runs").join(run_id)
}

/// Heuristic: is this project path an obvious throwaway/scratch location — a
/// system temp directory — rather than a real project the user works in?
///
/// Used to keep the dashboard's project list uncluttered. Deliberately
/// **path-based, not name-based**, so it never hides a legitimately-named
/// project like `my-test-app`; only paths under a temp root are treated as
/// scratch. Users can still see everything by setting
/// `TRACE_SHOW_ALL_PROJECTS=1` (honored at the API boundary, not here).
pub fn is_scratch_project_path(path: &str) -> bool {
    let p = path.replace('\\', "/");
    const TEMP_MARKERS: &[&str] = &[
        "/tmp/",
        "/private/tmp/",
        "/var/folders/",
        "/private/var/folders/",
    ];
    if TEMP_MARKERS.iter().any(|m| p.contains(m)) {
        return true;
    }
    // Anything under the OS-reported temp dir (covers Windows %TEMP%, etc.).
    if let Some(tmp) = std::env::temp_dir().to_str() {
        let tmp = tmp.replace('\\', "/");
        if !tmp.is_empty() && p.starts_with(&tmp) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_scratch_project_path;

    #[test]
    fn scratch_paths_are_temp_dirs_not_names() {
        // Temp locations → scratch.
        assert!(is_scratch_project_path("/tmp/swimlane-test"));
        assert!(is_scratch_project_path("/private/tmp/trace-live-demo"));
        assert!(is_scratch_project_path("/var/folders/xy/abc/T/scratch"));
        // Real project locations → kept, even with "test" in the NAME.
        assert!(!is_scratch_project_path("/Users/me/Desktop/simAPI"));
        assert!(!is_scratch_project_path("/Users/me/projects/my-test-app"));
        assert!(!is_scratch_project_path("/home/me/work/portfolio"));
    }
}
