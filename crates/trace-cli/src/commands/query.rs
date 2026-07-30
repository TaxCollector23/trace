//! Read-only CLI views that mirror the dashboard: runs, show, patch, risks,
//! costs, checkpoints. Each queries the local daemon.

use anyhow::Result;
use serde_json::Value;
use trace_core::models::*;

use crate::client::Client;
use crate::colors;
use crate::daemon_ctl;

fn client() -> Result<Client> {
    Ok(Client::new(daemon_ctl::ensure_running()?))
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s.to_string()
    }
}

/// `trace runs` — list recent runs.
pub fn runs() -> Result<()> {
    let c = client()?;
    let runs: Vec<RunSummary> = c.get_json("/api/runs?limit=50")?;
    if runs.is_empty() {
        println!("No runs yet. Try `trace run \"claude ...\"`.");
        return Ok(());
    }
    println!(
        "{}",
        colors::bold("STATUS      PROJECT     FILES  SECRETS  COMMAND")
    );
    for r in runs {
        let status = colors::status_padded(&r.run.status, 10);
        println!(
            "{}  {:<10}  {:>5}  {:>7}  {}  ({})",
            status,
            short(&r.project_name, 10),
            r.files_changed,
            r.secret_warnings,
            short(&r.run.command, 48),
            short(&r.run.id, 8),
        );
    }
    Ok(())
}

/// `trace show <run_id>` — run summary + timeline.
pub fn show(run_id: &str) -> Result<()> {
    let c = client()?;
    let r: RunSummary = c.get_json(&format!("/api/runs/{run_id}"))?;
    println!("{}", colors::bold(&format!("Run {}", r.run.id)));
    println!("  command:  {}", r.run.command);
    println!("  project:  {}", r.project_name);
    println!(
        "  status:   {} (exit {:?})",
        colors::status(&r.run.status),
        r.run.exit_code
    );
    println!("  started:  {}", r.run.started_at);
    println!(
        "  ended:    {}",
        r.run.ended_at.unwrap_or_else(|| "—".into())
    );
    println!(
        "  files:    {}   commands: {}   secrets: {}",
        r.files_changed, r.command_count, r.secret_warnings
    );
    if let Some(cost) = r.estimated_cost {
        println!("  cost:     ${cost:.4} (estimated)");
    }
    let events: Vec<Event> = c.get_json(&format!("/api/runs/{run_id}/timeline"))?;
    println!("\n{}", colors::bold("Timeline:"));
    for e in events {
        println!(
            "  {}  [{}] {}",
            colors::dim(&e.created_at),
            e.event_type,
            e.message
        );
    }
    Ok(())
}

/// `trace patch <run_id>` — changed files.
pub fn patch(run_id: &str) -> Result<()> {
    let c = client()?;
    let changes: Vec<FileChange> = c.get_json(&format!("/api/runs/{run_id}/file-changes"))?;
    if changes.is_empty() {
        println!("No file changes recorded for this run.");
        return Ok(());
    }
    for ch in changes {
        let tone = match ch.change_type.as_str() {
            "created" => colors::green(&format!("{:<9}", ch.change_type)),
            "deleted" => colors::red(&format!("{:<9}", ch.change_type)),
            _ => colors::yellow(&format!("{:<9}", ch.change_type)),
        };
        println!(
            "  {} {}  {}",
            tone,
            ch.path,
            ch.diff_summary.unwrap_or_default()
        );
    }
    Ok(())
}

/// `trace risks <run_id>` — guarded commands + secret warnings (redacted).
pub fn risks(run_id: &str) -> Result<()> {
    let c = client()?;
    let cmds: Vec<CommandRecord> = c.get_json(&format!("/api/runs/{run_id}/commands"))?;
    let guarded: Vec<_> = cmds
        .into_iter()
        .filter(|c| matches!(c.decision.as_str(), "block" | "warn" | "require_approval"))
        .collect();
    println!("{}", colors::bold("Command decisions:"));
    if guarded.is_empty() {
        println!("  {}", colors::dim("(none)"));
    }
    for c in guarded {
        println!("  [{}] {}", colors::status(&c.decision), c.command);
    }
    let secrets: Vec<SecretRecord> = c.get_json(&format!("/api/runs/{run_id}/secrets"))?;
    println!(
        "\n{}",
        colors::bold("Secret / protected-file warnings (redacted):")
    );
    if secrets.is_empty() {
        println!("  {}", colors::dim("(none)"));
    }
    for s in secrets {
        println!(
            "  {}  {}  {}",
            colors::yellow(&s.secret_type),
            s.redacted_value,
            s.file_path.unwrap_or_else(|| "(output/diff)".into())
        );
    }
    Ok(())
}

/// `trace costs <run_id>` — API usage + estimated cost.
pub fn costs(run_id: &str) -> Result<()> {
    let c = client()?;
    let resp: Value = c.get_json(&format!("/api/runs/{run_id}/cost"))?;
    let usage = resp
        .get("usage")
        .and_then(|u| u.as_array())
        .cloned()
        .unwrap_or_default();
    if usage.is_empty() {
        println!(
            "{}",
            colors::dim("No API usage recorded for this run (cost unavailable).")
        );
        return Ok(());
    }
    println!(
        "{}",
        colors::bold("PROVIDER    MODEL                IN     OUT    COST")
    );
    for u in &usage {
        println!(
            "{:<10}  {:<18}  {:>5}  {:>5}  {}",
            u["provider"].as_str().unwrap_or("?"),
            u["model"].as_str().unwrap_or("?"),
            u["input_tokens"],
            u["output_tokens"],
            colors::green(&format!(
                "${:.4}",
                u["estimated_cost"].as_f64().unwrap_or(0.0)
            )),
        );
    }
    if let Some(total) = resp.get("total_estimated").and_then(|t| t.as_f64()) {
        println!(
            "\n{} {}",
            colors::bold("total estimated:"),
            colors::green(&format!("${total:.4}"))
        );
    }
    Ok(())
}

/// `trace checkpoints` — checkpoints across recent runs (for rollback).
pub fn checkpoints() -> Result<()> {
    let c = client()?;
    let runs: Vec<RunSummary> = c.get_json("/api/runs?limit=50")?;
    let mut found = false;
    for r in runs {
        let cps: Vec<Checkpoint> = c.get_json(&format!("/api/runs/{}/checkpoints", r.run.id))?;
        for cp in cps.into_iter().filter(|c| c.git_ref.is_some()) {
            found = true;
            println!(
                "  {}  {}  {}  ({})",
                colors::dim(&cp.created_at),
                cp.checkpoint_type,
                colors::brand(&short(cp.git_ref.as_deref().unwrap_or(""), 12)),
                short(&r.run.command, 40),
            );
        }
    }
    if !found {
        println!("{}", colors::dim("No checkpoints with a Git ref yet."));
    }
    Ok(())
}
