//! `trg init` — initialize TraceGuard in the current project.

use anyhow::{Context, Result};
use traceguard_core::models::NewProject;
use traceguard_core::{git, paths, time::now_rfc3339, ProjectConfig};

use crate::client::Client;
use crate::daemon_ctl;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    let project_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    let config_path = paths::project_config_path(&cwd);
    if config_path.exists() {
        println!("TraceGuard is already initialized in this project.");
        println!("  config: {}", config_path.display());
        return Ok(());
    }

    // Write the per-project config and folder.
    let config = ProjectConfig::new(project_name.clone(), now_rfc3339());
    config.save(&config_path)?;

    // Keep captured run logs out of the user's git history. Config can still be
    // committed if the user wants to share check/protection rules.
    let gitignore = paths::project_dir(&cwd).join(".gitignore");
    std::fs::write(&gitignore, "runs/\n").ok();

    // Register in the global database via the daemon (starting it if needed).
    let port = daemon_ctl::ensure_running()?;
    let client = Client::new(port);
    let _: serde_json::Value = client.post_json(
        "/api/projects",
        &NewProject {
            name: project_name.clone(),
            path: cwd.display().to_string(),
            config_path: config_path.display().to_string(),
        },
    )?;

    println!("Initialized TraceGuard project \"{project_name}\".");
    println!("  config:   {}", config_path.display());
    println!("  database: {}", paths::database_path()?.display());

    if git::is_git_repo(&cwd) {
        let state = git::capture_state(&cwd);
        if state.dirty {
            println!("\nNote: the working tree has uncommitted changes.");
            println!("Runs will record that the starting state was dirty.");
        }
    } else {
        println!("\nWarning: this folder is not a Git repository.");
        println!("Checkpoints and rollback require Git. TraceGuard will NOT");
        println!("initialize Git for you — run `git init` yourself if you want");
        println!("checkpoint/rollback support.");
    }

    println!("\nNext: run an agent through TraceGuard, e.g. `trg run \"claude\"`.");
    Ok(())
}
