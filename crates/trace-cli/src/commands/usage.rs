//! The first CLI consumer of canonical imported usage events.

use anyhow::{bail, Result};
use trace_core::usage::{UsageGroup, UsageResponse};

use crate::client::Client;
use crate::colors;
use crate::daemon_ctl;

pub fn run(by: &str, since: Option<&str>, json: bool) -> Result<()> {
    if UsageGroup::parse(by).is_none() {
        bail!("unknown usage grouping {by:?}");
    }
    let client = Client::new(daemon_ctl::ensure_running()?);
    let mut path = format!("/api/usage?by={}", query_escape(by));
    if let Some(since) = since {
        path.push_str("&since=");
        path.push_str(&query_escape(since));
    }
    let response: UsageResponse = client.get_json(&path)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human(&response);
    }
    Ok(())
}

fn print_human(response: &UsageResponse) {
    println!(
        "{} (by {}, imported {} file{})",
        colors::bold("Claude Code usage"),
        response.by,
        response.import.files_seen,
        if response.import.files_seen == 1 {
            ""
        } else {
            "s"
        }
    );
    println!(
        "  events: {}   input: {}   output: {}   cost: {}",
        response.totals.event_count,
        token(response.totals.input_tokens),
        token(response.totals.output_tokens),
        cost(response.totals.cost_usd),
    );
    if response.totals.unavailable_cost_events > 0 {
        println!(
            "  cost unavailable for {} event{}",
            response.totals.unavailable_cost_events,
            if response.totals.unavailable_cost_events == 1 {
                ""
            } else {
                "s"
            }
        );
    }
    if response.import.malformed_lines > 0 {
        println!("  malformed lines: {}", response.import.malformed_lines);
    }

    if response.rows.is_empty() {
        println!("\n{}", colors::dim("No Claude Code usage events found."));
        return;
    }
    println!(
        "\n{}",
        colors::bold("KEY                         EVENTS   INPUT       OUTPUT      COST")
    );
    for row in &response.rows {
        println!(
            "{:<27} {:>6}   {:>10}  {:>10}  {}",
            truncate(&row.key, 27),
            row.totals.event_count,
            token(row.totals.input_tokens),
            token(row.totals.output_tokens),
            cost(row.totals.cost_usd),
        );
    }
}

fn token(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn cost(value: Option<f64>) -> String {
    value
        .map(|value| format!("{}{}", "$", format!("{:.4}", value)))
        .unwrap_or_else(|| "unavailable".to_string())
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>()
        + "…"
}

/// Keep timestamps and other user-provided query values out of URL delimiter
/// syntax without adding a CLI-only URL dependency.
fn query_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            escaped.push(byte as char);
        } else {
            escaped.push('%');
            escaped.push(hex(byte >> 4));
            escaped.push(hex(byte & 0x0f));
        }
    }
    escaped
}

fn hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!(),
    }
}
