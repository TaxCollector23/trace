//! Aider Markdown history importer.
//!
//! Aider's documented default history is `.aider.chat.history.md`. The file
//! is human-readable Markdown, not a stable usage API. Trace therefore only
//! parses the explicit `Tokens: ... sent, ... received` and `Cost: $...`
//! summaries when present; all other assistant turns remain unknown rather
//! than estimated.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::Store;
use crate::models::NewUsageEvent;

use super::{
    cost_source, empty_report, insert_event, merge_file, record_source, source_ref, FileStats,
    ImportContext, ImportReport, Importer,
};

#[derive(Debug, Clone, Copy)]
pub struct AiderImporter;

impl Importer for AiderImporter {
    fn id(&self) -> &'static str {
        "aider"
    }

    fn display_name(&self) -> &'static str {
        "Aider"
    }

    fn discover(&self, context: &ImportContext) -> Vec<PathBuf> {
        let is_history = |path: &Path| {
            path.file_name()
                .is_some_and(|name| name == ".aider.chat.history.md")
        };
        let mut found = Vec::new();
        let home_history = context.home.join(".aider.chat.history.md");
        if home_history.is_file() {
            found.push(home_history);
        }
        if let Ok(current_dir) = std::env::current_dir() {
            if current_dir != context.home {
                found.extend(super::walk_files(&current_dir, is_history));
            }
        }
        found.sort();
        found.dedup();
        found
    }

    fn import(&self, store: &Store, context: &ImportContext) -> Result<ImportReport> {
        let paths = self.discover(context);
        let mut report = empty_report(self, paths.clone());
        for path in paths {
            let stats = import_history(store, &path)
                .with_context(|| format!("importing Aider history {}", path.display()))?;
            merge_file(&mut report, &path, &stats);
            record_source(store, self.id(), &path, &stats)?;
        }
        Ok(report)
    }
}

#[derive(Debug, Default)]
struct Turn {
    heading_offset: i64,
    locator: i64,
    session_ts: Option<String>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cost_usd: Option<f64>,
    has_content: bool,
}

fn import_history(store: &Store, path: &Path) -> Result<FileStats> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut stats = FileStats {
        next_offset: bytes.len().min(i64::MAX as usize) as i64,
        ..FileStats::default()
    };
    let session_id = format!("aider:{}", path.display());
    let mut current: Option<Turn> = None;
    let mut line_offset = 0i64;

    for line in text.split_inclusive('\n') {
        let clean = line.trim_end_matches(['\n', '\r']);
        if let Some(start) = clean.strip_prefix("# aider chat started at ") {
            flush_turn(store, path, &session_id, &mut current, &mut stats)?;
            current = Some(Turn {
                heading_offset: line_offset,
                locator: line_offset,
                session_ts: parse_timestamp(start),
                ..Turn::default()
            });
        } else if clean.starts_with("#### ") {
            flush_turn(store, path, &session_id, &mut current, &mut stats)?;
            current = Some(Turn {
                heading_offset: line_offset,
                locator: line_offset,
                ..Turn::default()
            });
        } else if let Some(turn) = current.as_mut() {
            let trimmed = clean.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("> Tokens:")
                || trimmed.starts_with("> Cost:")
            {
                if let Some((input, output)) = parse_tokens(trimmed) {
                    turn.input_tokens = Some(input);
                    turn.output_tokens = Some(output);
                    turn.locator = line_offset;
                } else if trimmed.starts_with("> Tokens:") {
                    stats.parse_errors += 1;
                }
                if let Some(cost) = parse_cost(trimmed) {
                    turn.cost_usd = Some(cost);
                    turn.locator = line_offset;
                }
            } else if let Some(model) = trimmed.strip_prefix("> Model:") {
                let model = model.trim();
                if !model.is_empty() {
                    turn.model = Some(model.to_string());
                }
            } else if !trimmed.starts_with("# aider chat started at ") {
                // Content existence only; the content itself is never copied.
                turn.has_content = true;
            }
        }
        line_offset += line.len().min(i64::MAX as usize) as i64;
    }
    flush_turn(store, path, &session_id, &mut current, &mut stats)?;
    Ok(stats)
}

fn flush_turn(
    store: &Store,
    path: &Path,
    session_id: &str,
    current: &mut Option<Turn>,
    stats: &mut FileStats,
) -> Result<()> {
    let Some(turn) = current.take() else {
        return Ok(());
    };
    if !turn.has_content
        && turn.input_tokens.is_none()
        && turn.output_tokens.is_none()
        && turn.cost_usd.is_none()
    {
        return Ok(());
    }
    let message_id = format!("turn:{}", turn.heading_offset);
    let cost_usd = turn.cost_usd;
    insert_event(
        store,
        NewUsageEvent {
            run_id: None,
            agent: "aider".to_string(),
            session_id: session_id.to_string(),
            message_id,
            parent_message_id: None,
            is_subagent: false,
            subagent_id: None,
            model: turn.model,
            input_tokens: turn.input_tokens,
            output_tokens: turn.output_tokens,
            cache_read_tokens: None,
            cache_write_tokens: None,
            cache_write_5m_tokens: None,
            cache_write_1h_tokens: None,
            reasoning_tokens: None,
            ts: turn.session_ts,
            cost_usd,
            cost_source: cost_source(cost_usd),
            raw_ref: source_ref(path, &format!("byte={}", turn.locator)),
        },
        stats,
    )
}

fn parse_timestamp(value: &str) -> Option<String> {
    // Aider's default history uses a timezone-less local timestamp. Keeping it
    // unavailable is safer than silently assigning the machine's timezone.
    if chrono::DateTime::parse_from_rfc3339(value).is_ok() {
        Some(value.to_string())
    } else {
        None
    }
}

fn parse_tokens(line: &str) -> Option<(i64, i64)> {
    let body = line.strip_prefix("> Tokens:")?.trim();
    let sent = parse_labeled_count(body, "sent")?;
    let received = parse_labeled_count(body, "received")?;
    Some((sent, received))
}

fn parse_labeled_count(body: &str, label: &str) -> Option<i64> {
    let end = body.to_ascii_lowercase().find(label)?;
    let prefix = body[..end]
        .trim_end_matches(|character: char| character.is_ascii_whitespace() || character == ',');
    let start = prefix
        .char_indices()
        .rev()
        .find(|(_, character)| {
            !character.is_ascii_digit()
                && *character != ','
                && *character != '.'
                && *character != 'k'
                && *character != 'K'
                && *character != 'm'
                && *character != 'M'
        })
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(0);
    parse_count(prefix[start..].trim())
}

fn parse_count(value: &str) -> Option<i64> {
    let cleaned = value.trim().trim_end_matches('.').replace(',', "");
    let (number, multiplier) = match cleaned.chars().last()? {
        'k' | 'K' => (&cleaned[..cleaned.len() - 1], 1_000.0),
        'm' | 'M' => (&cleaned[..cleaned.len() - 1], 1_000_000.0),
        _ => (cleaned.as_str(), 1.0),
    };
    let parsed = number.parse::<f64>().ok()?;
    if !parsed.is_finite() || parsed < 0.0 {
        return None;
    }
    let total = (parsed * multiplier).round();
    (total <= i64::MAX as f64).then_some(total as i64)
}

fn parse_cost(line: &str) -> Option<f64> {
    let body = line.strip_prefix("> Cost:")?;
    let dollar = body.find('$')?;
    let value = body[dollar + 1..]
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect::<String>();
    let cost = value.parse::<f64>().ok()?;
    cost.is_finite().then_some(cost)
}

pub fn import_path(store: &Store, path: &Path) -> Result<ImportReport> {
    let importer = AiderImporter;
    let mut report = empty_report(&importer, vec![path.to_path_buf()]);
    let stats =
        import_history(store, path).with_context(|| format!("importing {}", path.display()))?;
    merge_file(&mut report, path, &stats);
    record_source(store, importer.id(), path, &stats)?;
    Ok(report)
}
