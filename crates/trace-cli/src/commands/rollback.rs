//! `trc rollback` — restore a checkpoint via git, with confirmation.
//!
//! Rollback is always Git-based and never destructive without an explicit
//! confirmation from the user.

use std::io::Write;

use anyhow::{Context, Result};
use trace_core::models::{Checkpoint, RunSummary};

use crate::client::Client;
use crate::daemon_ctl;

pub fn run(yes: bool) -> Result<()> {
    let port = daemon_ctl::ensure_running()?;
    let client = Client::new(port);

    // Fetch recent runs and pick a target.
    let runs: Vec<RunSummary> = client.get_json("/api/runs?limit=25")?;
    if runs.is_empty() {
        println!("No runs recorded yet — nothing to roll back to.");
        return Ok(());
    }

    // Find the most recent run that has a checkpoint with a git ref.
    let target = select_run(&client, &runs)?;
    let Some((run, checkpoint)) = target else {
        println!("No checkpoints with a Git reference were found.");
        println!("Rollback requires a Git repository and a checkpoint.");
        return Ok(());
    };

    println!("\nRollback target:");
    println!("  run:        {}", run.run.command);
    println!("  status:     {}", run.run.status);
    println!(
        "  checkpoint: {} ({})",
        short(checkpoint.git_ref.as_deref().unwrap_or("")),
        checkpoint.checkpoint_type
    );
    println!("  created:    {}", checkpoint.created_at);
    println!("\nThis will restore your working tree to the checkpoint above.");

    if !yes && !confirm("Proceed with rollback?")? {
        println!("Rollback cancelled.");
        return Ok(());
    }

    let resp: serde_json::Value = client
        .post_json(
            &format!("/api/runs/{}/rollback", run.run.id),
            &serde_json::json!({}),
        )
        .context("requesting rollback from daemon")?;

    if resp.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        println!("Rollback completed.");
    } else {
        println!("Rollback response: {resp}");
    }
    Ok(())
}

/// Pick the most recent run that has a usable checkpoint.
fn select_run(client: &Client, runs: &[RunSummary]) -> Result<Option<(RunSummary, Checkpoint)>> {
    for run in runs {
        let checkpoints: Vec<Checkpoint> =
            client.get_json(&format!("/api/runs/{}/checkpoints", run.run.id))?;
        if let Some(cp) = checkpoints.into_iter().rev().find(|c| c.git_ref.is_some()) {
            return Ok(Some((run.clone(), cp)));
        }
    }
    Ok(None)
}

fn confirm(question: &str) -> Result<bool> {
    print!("{question} [y/N]: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(is_affirmative(&input))
}

/// Whether a prompt answer means "yes". Defaults to NO for anything unclear,
/// so rollback (which mutates the working tree) never proceeds on ambiguous
/// input like Enter, "yep", or a stray word.
fn is_affirmative(input: &str) -> bool {
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn short(git_ref: &str) -> String {
    git_ref.chars().take(10).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_y_and_yes_confirm() {
        for yes in ["y", "Y", "yes", "YES", "Yes", "  y  ", "yes\n"] {
            assert!(is_affirmative(yes), "{yes:?} should confirm");
        }
    }

    #[test]
    fn everything_else_cancels() {
        // Enter, explicit no, and anything ambiguous must NOT roll back.
        for no in ["", "\n", "n", "no", "nope", "yep", "sure", "1", "yay"] {
            assert!(!is_affirmative(no), "{no:?} must not confirm a rollback");
        }
    }

    #[test]
    fn short_ref_is_ten_chars() {
        assert_eq!(short("0123456789abcdef"), "0123456789");
        assert_eq!(short("abc"), "abc");
    }
}
