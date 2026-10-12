//! `trc verify --day` — put Trace's local accounting beside the provider check.
//!
//! This command does not claim to reconcile a provider bill. It selects the
//! exact day bucket from Trace's local transcript import and tells the user
//! where to compare it in the Anthropic console, with the known reasons the
//! two figures can differ.

use anyhow::Result;
use chrono::NaiveDate;
use trace_core::usage::{UsageResponse, UsageTotals};

use crate::client::Client;
use crate::daemon_ctl;

pub fn run(day: &str) -> Result<()> {
    let parsed = NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("--day must be YYYY-MM-DD, got {day:?}"))?;
    let canonical_day = parsed.format("%Y-%m-%d").to_string();
    let port = daemon_ctl::ensure_running()?;
    let client = Client::new(port);
    let response: UsageResponse = client.get_json(&format!(
        "/api/usage?by=day&since={}",
        query_escape(&canonical_day)
    ))?;

    println!("Trace usage verification — {canonical_day}");
    match response.rows.iter().find(|row| row.key == canonical_day) {
        Some(row) => print_totals(&row.totals),
        None => {
            println!("  events: 0");
            println!("  cost: unavailable (no local usage event was recorded for this day)");
        }
    }
    println!("  imported files: {}", response.import.files_seen);
    if response.import.malformed_lines > 0 {
        println!(
            "  malformed lines skipped: {}",
            response.import.malformed_lines
        );
    }

    println!();
    println!("Provider comparison (Claude Code):");
    println!("  Open the Anthropic Console usage page and select {canonical_day}:");
    println!("  https://console.anthropic.com/settings/usage");
    println!("  Compare API usage for the same workspace, account, and time zone.");
    println!();
    println!("Known sources of drift:");
    println!("  - local transcript retention and provider console aggregation windows");
    println!(
        "  - duplicate streaming records, which Trace deduplicates by session/message identity"
    );
    println!("  - cache TTLs, long-context tiers, provider routing, and rounding");
    println!("  - plan/console totals are not necessarily public API-rate computed totals");
    println!("  - unknown model pricing remains unavailable instead of being treated as zero");
    Ok(())
}

fn print_totals(totals: &UsageTotals) {
    println!("  events: {}", totals.event_count);
    println!("  input tokens: {}", token(totals.input_tokens));
    println!("  output tokens: {}", token(totals.output_tokens));
    println!("  cache read tokens: {}", token(totals.cache_read_tokens));
    println!("  cache write tokens: {}", token(totals.cache_write_tokens));
    println!("  subagent events: {}", totals.subagent_events);
    match totals.cost_usd {
        Some(cost) => println!("  cost: ${cost:.8} ({})", totals.cost_source),
        None => println!("  cost: unavailable ({})", totals.cost_source),
    }
    if totals.unavailable_cost_events > 0 {
        println!(
            "  unavailable cost events: {}",
            totals.unavailable_cost_events
        );
    }
}

fn token(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

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

#[cfg(test)]
mod tests {
    use super::query_escape;

    #[test]
    fn query_escape_keeps_dates_and_encodes_delimiters() {
        assert_eq!(query_escape("2026-10-12"), "2026-10-12");
        assert_eq!(
            query_escape("2026-10-12T00:00:00Z"),
            "2026-10-12T00%3A00%3A00Z"
        );
    }
}
