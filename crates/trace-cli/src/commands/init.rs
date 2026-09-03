//! `trc init` — initialize Trace in the current project.
//!
//! Inspects the folder Trace is run inside: finds the repository root, detects
//! whether it's a Git repo, reads the `origin` remote and resolves the GitHub
//! `owner/repo` when there is one, notes a `.github/` directory, and records the
//! branch. Only what Trace actually needs is persisted (project name + the
//! detected GitHub repo/branch) to `.trace/config.toml`. Everything degrades
//! gracefully — a folder with no Git or no GitHub remote still initializes.

use anyhow::{Context, Result};
use trace_core::models::NewProject;
use trace_core::{git, github, paths, time::now_rfc3339, ProjectConfig};

use crate::client::Client;
use crate::colors;
use crate::daemon_ctl;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;

    // 1. Detect the project root: the git top level if we're in a repo,
    //    otherwise the current directory.
    let root = git::repo_root(&cwd).unwrap_or_else(|| cwd.clone());
    let project_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    let config_path = paths::project_config_path(&root);
    if config_path.exists() {
        println!("Trace is already initialized here ({project_name}).");
        println!("  {}", colors::dim(&config_path.display().to_string()));
        return Ok(());
    }

    // 2-4. Inspect the repository. Capture the dirty check BEFORE writing
    // anything under .trace/ — writing config.toml/.gitignore first would
    // make init's own untracked directory register as "uncommitted changes"
    // in the very check meant to report the repo's pre-existing state.
    let is_git = git::is_git_repo(&root);
    let was_dirty = is_git && git::capture_state(&root).dirty;
    let github_repo = git::remote_url(&root)
        .as_deref()
        .and_then(github::parse_remote)
        .map(|r| format!("{}/{}", r.owner, r.repo));
    let branch = if is_git {
        git::current_branch(&root)
    } else {
        None
    };
    let dot_github = root.join(".github");
    let workflows = count_workflows(&dot_github);

    // 5-7. Persist only what Trace needs.
    let mut config = ProjectConfig::new(project_name.clone(), now_rfc3339());
    config.github_repo = github_repo.clone();
    config.default_branch = branch.clone();
    config.save(&config_path)?;

    // Keep the whole .trace/ directory (including this .gitignore) out of the
    // user's git history — `*` here is relative to .trace/ itself, so it
    // covers config.toml, runs/, and any future files without needing the
    // parent repo's own .gitignore touched.
    let gitignore = paths::project_dir(&root).join(".gitignore");
    std::fs::write(&gitignore, "*\n").ok();

    // Register in the global database via the daemon (starting it if needed).
    let port = daemon_ctl::ensure_running()?;
    let client = Client::new(port);
    let _: serde_json::Value = client.post_json(
        "/api/projects",
        &NewProject {
            name: project_name.clone(),
            path: root.display().to_string(),
            config_path: config_path.display().to_string(),
        },
    )?;

    // --- Friendly summary of what Trace found and connected ---
    println!(
        "{} Initialized Trace in {}",
        colors::green("✓"),
        colors::bold(&project_name)
    );
    println!("  {}", colors::dim(&root.display().to_string()));
    println!();

    match &github_repo {
        Some(repo) => {
            println!("  {}  {}", colors::dim("GitHub"), repo);
            if let Some(b) = &branch {
                println!("  {}  {}", colors::dim("Branch"), b);
            }
        }
        None if is_git => {
            println!(
                "  {}  {}",
                colors::dim("GitHub"),
                colors::dim("no origin remote — local-only project (that's fine)")
            );
        }
        None => {
            println!(
                "  {}  {}",
                colors::dim("Git"),
                colors::dim(
                    "not a Git repo — checkpoints/rollback need Git; run `git init` when ready"
                )
            );
        }
    }
    if let Some(n) = workflows {
        println!(
            "  {}  {} workflow{} detected in .github/",
            colors::dim("CI"),
            n,
            if n == 1 { "" } else { "s" }
        );
    }

    if was_dirty {
        println!(
            "\n  {}",
            colors::dim("Working tree has uncommitted changes — runs will note the starting state was dirty.")
        );
    }

    println!(
        "\nNext:\n  1. {}  connect your agent\n  2. {}  run it under Trace\n  3. {}  watch it live",
        colors::bold("trc install agents"),
        colors::bold("trc run \"<your agent command>\""),
        colors::bold("trc dashboard"),
    );
    Ok(())
}

/// Count workflow files in `.github/workflows/`, if the directory exists.
/// Returns `None` when there's no `.github` at all.
fn count_workflows(dot_github: &std::path::Path) -> Option<usize> {
    if !dot_github.is_dir() {
        return None;
    }
    let wf = dot_github.join("workflows");
    let count = std::fs::read_dir(&wf)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|x| x == "yml" || x == "yaml")
                })
                .count()
        })
        .unwrap_or(0);
    Some(count)
}
