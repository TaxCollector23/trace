//! Global and per-project filesystem locations used by Trace.
//!
//! All global state lives under `~/.trace`. Per-project state lives under
//! `<project>/.trace`. Everything is local to the machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Environment override for the global Trace home directory. When set (and
/// non-empty), it replaces the default `~/.trace` everywhere the CLI, daemon,
/// and core resolve global state. This is the seam that keeps test/CI/dev data
/// from polluting a developer's real `~/.trace` store.
pub const HOME_ENV: &str = "TRACE_HOME";

/// Environment override for the SQLite database path specifically. Takes
/// precedence over `TRACE_HOME` for the database file only (the run-log dir,
/// daemon state, etc. still resolve under `home()`).
pub const DB_ENV: &str = "TRACE_DB";

/// Root directory for all global Trace data.
///
/// Defaults to `~/.trace`, but honors the `TRACE_HOME` env override so tests,
/// CI, and development can point at a disposable directory instead of the
/// user's real store. This is the single resolver every other global path is
/// built on.
pub fn home() -> Result<PathBuf> {
    if let Some(dir) = env_path(HOME_ENV) {
        return Ok(dir);
    }
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".trace"))
}

/// Backwards-compatible alias for [`home`]. Retained so existing call sites keep
/// working; both now honor `TRACE_HOME`.
pub fn global_dir() -> Result<PathBuf> {
    home()
}

/// Path to the global SQLite database.
///
/// Honors `TRACE_DB` (exact file path) if set, otherwise `<home>/trace.db`.
pub fn database_path() -> Result<PathBuf> {
    if let Some(db) = env_path(DB_ENV) {
        return Ok(db);
    }
    Ok(home()?.join("trace.db"))
}

/// Read an env var as a non-empty path, trimming surrounding whitespace.
fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var(var)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
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
    use super::*;
    use std::sync::Mutex;

    // Env vars are process-global; serialize the override tests so they don't
    // race each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn trace_home_override_redirects_home_and_db() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev_home = std::env::var(HOME_ENV).ok();
        let prev_db = std::env::var(DB_ENV).ok();
        std::env::remove_var(DB_ENV);
        std::env::set_var(HOME_ENV, "/tmp/trace-test-home");

        assert_eq!(home().unwrap(), PathBuf::from("/tmp/trace-test-home"));
        assert_eq!(
            database_path().unwrap(),
            PathBuf::from("/tmp/trace-test-home/trace.db")
        );
        // Other global paths follow home too.
        assert_eq!(
            daemon_state_path().unwrap(),
            PathBuf::from("/tmp/trace-test-home/daemon.json")
        );

        restore(HOME_ENV, prev_home);
        restore(DB_ENV, prev_db);
    }

    #[test]
    fn trace_db_override_takes_precedence_for_the_db_only() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev_home = std::env::var(HOME_ENV).ok();
        let prev_db = std::env::var(DB_ENV).ok();
        std::env::set_var(HOME_ENV, "/tmp/trace-test-home");
        std::env::set_var(DB_ENV, "/tmp/elsewhere/custom.db");

        assert_eq!(
            database_path().unwrap(),
            PathBuf::from("/tmp/elsewhere/custom.db")
        );
        // The home itself is unaffected by TRACE_DB.
        assert_eq!(home().unwrap(), PathBuf::from("/tmp/trace-test-home"));

        restore(HOME_ENV, prev_home);
        restore(DB_ENV, prev_db);
    }

    #[test]
    fn empty_env_override_is_ignored() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = std::env::var(HOME_ENV).ok();
        std::env::set_var(HOME_ENV, "   ");
        // Blank value falls through to the default (~/.trace), not an empty path.
        assert!(home().unwrap().ends_with(".trace"));
        restore(HOME_ENV, prev);
    }

    fn restore(var: &str, prev: Option<String>) {
        match prev {
            Some(v) => std::env::set_var(var, v),
            None => std::env::remove_var(var),
        }
    }

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
