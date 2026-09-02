//! `trc reset --local-data` — purge locally-stored run telemetry.
//!
//! Deletes runs/commands/events/checkpoints (and their child rows) from the
//! SQLite database, plus captured run-log directories, under the **active**
//! `TRACE_HOME` only. It never touches a path outside the active home, and
//! always shows exactly what will be deleted (row counts + paths) before doing
//! so. `--yes` skips the prompt for scripting; `--dry-run` reports and exits.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use trace_core::paths;
use trace_core::Store;

use crate::colors;

/// Options for `trc reset`.
pub struct ResetOptions {
    /// Purge local telemetry data. Required — `reset` refuses to run without an
    /// explicit target so it can grow other targets later without surprise.
    pub local_data: bool,
    /// Skip the confirmation prompt (for scripting/CI).
    pub yes: bool,
    /// Report what would be deleted, then exit without deleting anything.
    pub dry_run: bool,
}

pub fn run(opts: ResetOptions) -> Result<()> {
    if !opts.local_data {
        anyhow::bail!("nothing to reset; pass --local-data to purge local telemetry");
    }

    let home = paths::home()?;
    let db_path = paths::database_path()?;

    // Open the active DB (creating an empty one if none exists yet, so counts
    // are simply zero rather than an error).
    let store = Store::open(&db_path).context("opening the local database")?;
    let counts = store.telemetry_counts()?;

    // Run-log directories to remove — but only those that live *under* the
    // active TRACE_HOME. Per-project run logs outside the home are deliberately
    // left untouched (never delete outside the active home).
    let log_dirs = run_log_dirs_under_home(&home);

    println!("{}", colors::bold("Trace reset — local data"));
    println!("  home:        {}", home.display());
    println!("  database:    {}", db_path.display());
    println!("\n  Will delete from the database:");
    println!("    runs:        {}", counts.runs);
    println!("    commands:    {}", counts.commands);
    println!("    events:      {}", counts.events);
    println!("    checkpoints: {}", counts.checkpoints);
    if log_dirs.is_empty() {
        println!("\n  Run-log directories under home: none");
    } else {
        println!("\n  Will delete run-log directories under home:");
        for d in &log_dirs {
            println!("    {}", d.display());
        }
    }
    println!(
        "\n  Projects are preserved (no re-`trc init` needed). Nothing outside\n  {} is touched.",
        home.display()
    );

    if counts.is_empty() && log_dirs.is_empty() {
        println!("\nNothing to delete — the local store is already empty.");
        return Ok(());
    }

    if opts.dry_run {
        println!("\n{}", colors::dim("Dry run — nothing was deleted."));
        return Ok(());
    }

    if !opts.yes && !confirm("Permanently delete the data above?")? {
        println!("Reset cancelled.");
        return Ok(());
    }

    store
        .purge_local_data()
        .context("purging local telemetry")?;
    let mut removed_dirs = 0usize;
    for d in &log_dirs {
        match std::fs::remove_dir_all(d) {
            Ok(()) => removed_dirs += 1,
            Err(e) => eprintln!("  warning: could not remove {}: {e}", d.display()),
        }
    }

    println!(
        "\nDeleted {} run(s), {} command(s), {} event(s), {} checkpoint(s){}.",
        counts.runs,
        counts.commands,
        counts.events,
        counts.checkpoints,
        if removed_dirs > 0 {
            format!(" and {removed_dirs} run-log dir(s)")
        } else {
            String::new()
        }
    );
    Ok(())
}

/// Collect run-log directories that are located under the active home. Trace's
/// per-project run logs normally live at `<project>/.trace/runs`, which is
/// outside `~/.trace`; this only ever matches when `TRACE_HOME` has been
/// pointed at a directory that also contains the project (e.g. a disposable
/// test/CI sandbox). Any `<home>/**/.trace/runs` dir is included.
fn run_log_dirs_under_home(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    // A conventional global run-log location, if one is ever used.
    let global_runs = home.join("runs");
    if global_runs.is_dir() {
        out.push(global_runs);
    }
    collect_trace_runs(home, home, &mut out, 0);
    out.sort();
    out.dedup();
    out
}

/// Recursively find `.trace/runs` directories under `root`, bounded in depth so
/// this stays cheap. `home` is passed only to guard the invariant that every
/// returned path is inside the active home.
fn collect_trace_runs(dir: &Path, home: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some(".trace") {
            let runs = path.join("runs");
            // Invariant: never escape the active home.
            if runs.is_dir() && runs.starts_with(home) {
                out.push(runs);
            }
            continue;
        }
        collect_trace_runs(&path, home, out, depth + 1);
    }
}

fn confirm(question: &str) -> Result<bool> {
    print!("{question} [y/N]: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}
