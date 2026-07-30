//! Git inspection, checkpoint creation, diff parsing, and rollback.
//!
//! Uses the Git CLI directly (per spec) so behaviour matches what the developer
//! sees in their own terminal. All operations are best-effort and degrade
//! gracefully when the project is not a git repository.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use crate::models::ChangeType;

/// Snapshot of the repository state at a point in time.
#[derive(Debug, Clone)]
pub struct GitState {
    pub is_repo: bool,
    pub commit: Option<String>,
    pub dirty: bool,
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = run_git(cwd, args)?;
    if !out.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// True when `path` is inside a git working tree.
pub fn is_git_repo(path: &Path) -> bool {
    run_git(path, &["rev-parse", "--is-inside-work-tree"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The `origin` remote URL, if the project has one.
pub fn remote_url(path: &Path) -> Option<String> {
    let url = git_stdout(path, &["remote", "get-url", "origin"]).ok()?;
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

/// Capture the current HEAD commit and whether the tree is dirty.
pub fn capture_state(path: &Path) -> GitState {
    if !is_git_repo(path) {
        return GitState {
            is_repo: false,
            commit: None,
            dirty: false,
        };
    }
    let commit = git_stdout(path, &["rev-parse", "HEAD"]).ok();
    let dirty = run_git(path, &["status", "--porcelain"])
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    GitState {
        is_repo: true,
        commit,
        dirty,
    }
}

/// Create a lightweight checkpoint of the current working tree.
///
/// Returns a git object hash (from `git stash create`) when the tree is dirty,
/// or the HEAD commit when it is clean. This never disturbs the working tree, so
/// the wrapped command starts from exactly the state the user had.
pub fn create_checkpoint(path: &Path) -> Result<Option<String>> {
    if !is_git_repo(path) {
        return Ok(None);
    }
    // `stash create` builds a commit object representing the dirty state without
    // touching the index or working tree. Empty output means a clean tree.
    let stash = git_stdout(path, &["stash", "create", "traceguard checkpoint"]).unwrap_or_default();
    if !stash.is_empty() {
        return Ok(Some(stash));
    }
    // Clean tree: the checkpoint is simply HEAD.
    Ok(git_stdout(path, &["rev-parse", "HEAD"]).ok())
}

/// A single changed file from a git diff name-status.
#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub path: String,
    pub change_type: ChangeType,
    pub diff_summary: Option<String>,
}

fn parse_change_type(code: &str) -> ChangeType {
    match code.chars().next() {
        Some('A') => ChangeType::Created,
        Some('D') => ChangeType::Deleted,
        Some('R') => ChangeType::Renamed,
        _ => ChangeType::Modified,
    }
}

/// TraceGuard's own state directory must never appear in a project's diff.
fn is_internal(path: &str) -> bool {
    path == ".traceguard" || path.starts_with(".traceguard/")
}

/// Diff between `from_ref` (e.g. starting commit) and the current working tree,
/// including untracked files. Returns one entry per changed path. TraceGuard's
/// own `.traceguard/` directory is excluded.
pub fn diff_against(path: &Path, from_ref: &str) -> Result<Vec<DiffEntry>> {
    let mut entries: Vec<DiffEntry> = Vec::new();

    // Tracked changes (staged + unstaged) relative to the starting ref.
    let name_status = git_stdout(path, &["diff", "--name-status", "-M", from_ref])?;
    let numstat = git_stdout(path, &["diff", "--numstat", from_ref]).unwrap_or_default();
    let stat_map = parse_numstat(&numstat);

    for line in name_status.lines() {
        let mut parts = line.split('\t');
        let code = parts.next().unwrap_or("");
        // For renames the status line is `R<score>\told\tnew`; take the final field.
        let p = parts.next_back().unwrap_or("").to_string();
        if p.is_empty() || is_internal(&p) {
            continue;
        }
        entries.push(DiffEntry {
            change_type: parse_change_type(code),
            diff_summary: stat_map.get(&p).cloned(),
            path: p,
        });
    }

    // Untracked files are not part of `git diff`; surface them as created.
    let untracked =
        git_stdout(path, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();
    for p in untracked
        .lines()
        .filter(|l| !l.is_empty() && !is_internal(l))
    {
        if !entries.iter().any(|e| e.path == p) {
            entries.push(DiffEntry {
                path: p.to_string(),
                change_type: ChangeType::Created,
                diff_summary: Some("new untracked file".to_string()),
            });
        }
    }

    Ok(entries)
}

fn parse_numstat(numstat: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in numstat.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() == 3 {
            let added = parts[0];
            let removed = parts[1];
            let p = parts[2].to_string();
            map.insert(p, format!("+{added} -{removed}"));
        }
    }
    map
}

/// Full unified diff text between a ref and the working tree, for the patch view.
pub fn full_diff(path: &Path, from_ref: &str) -> Result<String> {
    git_stdout(path, &["diff", from_ref])
}

/// Restore the working tree to a checkpoint ref.
///
/// If the ref is a stash object it is applied; otherwise the tree is hard-reset
/// to the commit. Callers must confirm with the user before invoking this.
pub fn rollback_to(path: &Path, git_ref: &str) -> Result<()> {
    if !is_git_repo(path) {
        return Err(anyhow!("not a git repository; rollback requires git"));
    }
    // Determine whether the ref looks like a stash-created commit by checking if
    // it has the working-tree shape (a stash commit has 2-3 parents).
    let is_stash = git_stdout(path, &["rev-list", "--parents", "-n", "1", git_ref])
        .map(|s| s.split_whitespace().count() >= 3)
        .unwrap_or(false);

    if is_stash {
        let out = run_git(path, &["stash", "apply", git_ref])?;
        if !out.status.success() {
            return Err(anyhow!(
                "git stash apply failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    } else {
        let out = run_git(path, &["reset", "--hard", git_ref])?;
        if !out.status.success() {
            return Err(anyhow!(
                "git reset --hard failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    Ok(())
}
