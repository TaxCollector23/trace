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
    let stash = git_stdout(path, &["stash", "create", "trace checkpoint"]).unwrap_or_default();
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

/// Trace's own state directory must never appear in a project's diff.
fn is_internal(path: &str) -> bool {
    path == ".trace" || path.starts_with(".trace/")
}

/// Diff between `from_ref` (e.g. starting commit) and the current working tree,
/// including untracked files. Returns one entry per changed path. Trace's
/// own `.trace/` directory is excluded.
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

/// Splits a full unified diff (as produced by `full_diff`/`diff_range`) into
/// per-file patch text, keyed by the file's current path. Used wherever a
/// caller needs real patch content per file rather than just a stat summary
/// — the policy engine's regex checks (secrets, TODOs, swallowed catches,
/// etc.) only have something to match against once they see the actual
/// added/removed lines, not a "+12 -3" count.
pub fn split_diff_by_file(full_diff_text: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let mut current_path: Option<String> = None;
    let mut current_chunk = String::new();

    for line in full_diff_text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // Flush the previous file's chunk before starting a new one.
            if let Some(p) = current_path.take() {
                map.insert(p, std::mem::take(&mut current_chunk));
            }
            // Format: "a/<path> b/<path>" (paths match unless renamed, in
            // which case we still want the *new* path — the last token).
            current_path = rest
                .rsplit(' ')
                .next()
                .map(|s| s.trim_start_matches("b/").to_string());
        }
        current_chunk.push_str(line);
        current_chunk.push('\n');
    }
    if let Some(p) = current_path {
        map.insert(p, current_chunk);
    }
    map
}

/// Real per-file patch text for every file changed since `from_ref`,
/// against the current working tree. Complements `diff_against`, which only
/// returns a stat summary per file.
pub fn patches_by_file(
    path: &Path,
    from_ref: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let text = full_diff(path, from_ref)?;
    Ok(split_diff_by_file(&text))
}

/// Diff between two refs (e.g. `origin/main...HEAD`), independent of the
/// working tree — what CI uses to review a pull request from a plain
/// checkout, no GitHub API calls required. Mirrors `diff_against`'s output
/// shape so both paths can feed the same policy engine.
pub fn diff_range(path: &Path, range: &str) -> Result<Vec<DiffEntry>> {
    let mut entries: Vec<DiffEntry> = Vec::new();
    let name_status = git_stdout(path, &["diff", "--name-status", "-M", range])?;
    let numstat = git_stdout(path, &["diff", "--numstat", range]).unwrap_or_default();
    let stat_map = parse_numstat(&numstat);

    for line in name_status.lines() {
        let mut parts = line.split('\t');
        let code = parts.next().unwrap_or("");
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
    Ok(entries)
}

/// Real per-file patch text for a ref range — the range-diff counterpart to
/// `patches_by_file`.
pub fn patches_by_file_range(
    path: &Path,
    range: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let text = git_stdout(path, &["diff", range])?;
    Ok(split_diff_by_file(&text))
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

#[cfg(test)]
mod split_diff_tests {
    use super::split_diff_by_file;

    #[test]
    fn splits_multi_file_diff_by_final_path() {
        let diff = "diff --git a/src/foo.rs b/src/foo.rs\nindex 111..222 100644\n--- a/src/foo.rs\n+++ b/src/foo.rs\n@@ -1,2 +1,2 @@\n-old line\n+new line\ndiff --git a/src/bar.rs b/src/bar.rs\nindex 333..444 100644\n--- a/src/bar.rs\n+++ b/src/bar.rs\n@@ -1 +1 @@\n-bar old\n+bar new\n";
        let map = split_diff_by_file(diff);
        assert_eq!(map.len(), 2);
        assert!(map["src/foo.rs"].contains("+new line"));
        assert!(map["src/bar.rs"].contains("+bar new"));
        assert!(!map["src/foo.rs"].contains("bar"));
    }

    #[test]
    fn handles_renamed_file_using_new_path() {
        let diff = "diff --git a/old_name.rs b/new_name.rs\nsimilarity index 100%\nrename from old_name.rs\nrename to new_name.rs\n";
        let map = split_diff_by_file(diff);
        assert!(map.contains_key("new_name.rs"));
    }

    #[test]
    fn empty_diff_yields_empty_map() {
        assert!(split_diff_by_file("").is_empty());
    }
}

#[cfg(test)]
mod parser_tests {
    use super::{parse_change_type, parse_numstat};
    use crate::models::ChangeType;

    #[test]
    fn parse_change_type_maps_status_codes() {
        assert_eq!(parse_change_type("A"), ChangeType::Created);
        assert_eq!(parse_change_type("D"), ChangeType::Deleted);
        assert_eq!(parse_change_type("M"), ChangeType::Modified);
        // Renames carry a similarity score (e.g. `R100`); only the leading
        // letter is inspected.
        assert_eq!(parse_change_type("R"), ChangeType::Renamed);
        assert_eq!(parse_change_type("R100"), ChangeType::Renamed);
        // Copies and unknown/empty codes fall back to Modified.
        assert_eq!(parse_change_type("C075"), ChangeType::Modified);
        assert_eq!(parse_change_type(""), ChangeType::Modified);
    }

    #[test]
    fn parse_numstat_text_lines() {
        let numstat = "10\t2\tsrc/main.rs\n0\t5\tREADME.md\n";
        let map = parse_numstat(numstat);
        assert_eq!(map.get("src/main.rs").map(String::as_str), Some("+10 -2"));
        assert_eq!(map.get("README.md").map(String::as_str), Some("+0 -5"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn parse_numstat_binary_lines() {
        // Git emits `-\t-\t<path>` for binary files (no line counts). The parser
        // still records an entry, surfacing the `-`/`-` counts verbatim.
        let map = parse_numstat("-\t-\tassets/logo.png\n");
        assert_eq!(
            map.get("assets/logo.png").map(String::as_str),
            Some("+- --")
        );
    }

    #[test]
    fn parse_numstat_ignores_malformed_lines() {
        // Lines without exactly three tab-separated fields are skipped rather
        // than panicking (blank lines, headers, truncated output).
        let map = parse_numstat("garbage\n1\t2\ttoo\tmany\tfields\n\n3\t4\tok.rs\n");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("ok.rs").map(String::as_str), Some("+3 -4"));
    }

    #[test]
    fn parse_numstat_renamed_path_with_braces() {
        // `git diff --numstat` (without `-z`) emits a brace-expansion path for a
        // rename; the third field is stored as-is (keyed to that literal form).
        let map = parse_numstat("2\t1\tsrc/{old.rs => new.rs}\n");
        assert_eq!(
            map.get("src/{old.rs => new.rs}").map(String::as_str),
            Some("+2 -1")
        );
    }
}
