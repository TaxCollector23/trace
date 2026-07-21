//! `trace replay <run_id>` — replay a run's recorded events, commands, and
//! file changes in the order they actually happened, paced by their real
//! recorded timestamps (compressed so replay never drags on a long run).
//!
//! This reads the same data `trace show`/`patch`/`risks` read — nothing is
//! invented or simulated. It merges three real timelines into one and plays
//! them back.

use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use trace_core::models::*;

use crate::client::Client;
use crate::colors;
use crate::daemon_ctl;

fn client() -> Result<Client> {
    Ok(Client::new(daemon_ctl::ensure_running()?))
}

enum Item {
    Event(Event),
    Command(CommandRecord),
    FileChange(FileChange),
}

impl Item {
    fn created_at(&self) -> &str {
        match self {
            Item::Event(e) => &e.created_at,
            Item::Command(c) => &c.created_at,
            Item::FileChange(f) => &f.created_at,
        }
    }
}

/// `-y`/`--fast` skips pacing entirely — useful in scripts or CI.
pub fn run(run_id: &str, fast: bool) -> Result<()> {
    let c = client()?;
    let summary: RunSummary = c.get_json(&format!("/api/runs/{run_id}"))?;
    let events: Vec<Event> = c.get_json(&format!("/api/runs/{run_id}/timeline"))?;
    let commands: Vec<CommandRecord> = c.get_json(&format!("/api/runs/{run_id}/commands"))?;
    let changes: Vec<FileChange> = c.get_json(&format!("/api/runs/{run_id}/file-changes"))?;

    let mut items: Vec<Item> = Vec::new();
    items.extend(events.into_iter().map(Item::Event));
    items.extend(commands.into_iter().map(Item::Command));
    items.extend(changes.into_iter().map(Item::FileChange));
    items.sort_by(|a, b| a.created_at().cmp(b.created_at()));

    println!(
        "{}",
        colors::bold(&format!(
            "Replaying {} — {}",
            &summary.run.id[..8.min(summary.run.id.len())],
            summary.run.command
        ))
    );
    println!();

    if items.is_empty() {
        println!("{}", colors::dim("Nothing recorded for this run yet."));
        return Ok(());
    }

    let mut last: Option<DateTime<FixedOffset>> = None;
    for item in &items {
        let ts = DateTime::parse_from_rfc3339(item.created_at()).ok();
        if !fast {
            if let (Some(prev), Some(cur)) = (last, ts) {
                let real_gap = (cur - prev).to_std().unwrap_or(Duration::ZERO);
                let paced = real_gap.clamp(Duration::from_millis(150), Duration::from_millis(1500));
                sleep(paced);
            }
        }
        print_item(item);
        if ts.is_some() {
            last = ts;
        }
    }

    println!();
    println!(
        "{}",
        colors::bold(&format!(
            "── replay complete: {} (exit {}) ──",
            colors::status(&summary.run.status),
            summary
                .run
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into())
        ))
    );
    Ok(())
}

fn clock(created_at: &str) -> String {
    DateTime::parse_from_rfc3339(created_at)
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|_| created_at.to_string())
}

fn print_item(item: &Item) {
    match item {
        Item::Event(e) => {
            println!(
                "  {}  [{}] {}",
                colors::dim(&clock(&e.created_at)),
                e.event_type,
                e.message
            );
        }
        Item::Command(cmd) => {
            println!(
                "  {}  $ {}  [{}]",
                colors::dim(&clock(&cmd.created_at)),
                cmd.command,
                colors::status(&cmd.decision)
            );
        }
        Item::FileChange(f) => {
            let tone = match f.change_type.as_str() {
                "created" => colors::green(&f.change_type),
                "deleted" => colors::red(&f.change_type),
                _ => colors::yellow(&f.change_type),
            };
            let summary = f
                .diff_summary
                .as_deref()
                .map(|s| format!("  {s}"))
                .unwrap_or_default();
            println!(
                "  {}  {} {}{}",
                colors::dim(&clock(&f.created_at)),
                tone,
                f.path,
                summary
            );
        }
    }
}
