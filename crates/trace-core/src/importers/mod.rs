//! Read-only local session importers.
//!
//! Each importer owns the format-specific parser for one agent. The shared
//! contract is intentionally small: discover source files, emit canonical
//! [`NewUsageEvent`] rows without content, and persist a byte/row cursor plus
//! diagnostics. Pricing is not performed here.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::db::Store;
use crate::models::NewUsageEvent;

pub mod aider;
pub mod codex;
pub mod cursor;
pub mod opencode;

/// The public importer order is part of the onboarding contract.
pub const IMPORTER_IDS: &[&str] = &["codex", "opencode", "cursor", "aider"];

/// Filesystem roots used by importers. Tests can provide a disposable home;
/// production uses the user's actual home and never TRACE_HOME (which is the
/// Trace database home, not an agent's home).
#[derive(Debug, Clone)]
pub struct ImportContext {
    pub home: PathBuf,
}

impl ImportContext {
    pub fn from_environment() -> Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .context("could not determine the user home directory")?;
        Ok(Self { home })
    }

    pub fn with_home(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }
}

/// One import pass. Counts are intentionally separate so a UI can distinguish
/// malformed source lines from duplicate streaming records.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportReport {
    pub agent: String,
    pub files_found: usize,
    pub files_scanned: usize,
    pub events_seen: i64,
    pub events_imported: i64,
    pub duplicates: i64,
    pub parse_errors: i64,
    pub last_event_at: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FileStats {
    pub events_seen: i64,
    pub events_imported: i64,
    pub duplicates: i64,
    pub parse_errors: i64,
    pub last_event_at: Option<String>,
    pub next_offset: i64,
}

/// Format-specific importer interface. A new agent needs one implementation
/// file, fixtures, and a single registration in [`importer`].
pub trait Importer: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn discover(&self, context: &ImportContext) -> Vec<PathBuf>;
    fn import(&self, store: &Store, context: &ImportContext) -> Result<ImportReport>;
}

pub fn importer(id: &str) -> Option<Box<dyn Importer>> {
    match id {
        "codex" => Some(Box::new(codex::CodexImporter)),
        "opencode" => Some(Box::new(opencode::OpenCodeImporter)),
        "cursor" => Some(Box::new(cursor::CursorImporter)),
        "aider" => Some(Box::new(aider::AiderImporter)),
        _ => None,
    }
}

pub fn all_importers() -> Vec<Box<dyn Importer>> {
    IMPORTER_IDS.iter().filter_map(|id| importer(id)).collect()
}

pub fn import_agent(store: &Store, context: &ImportContext, agent: &str) -> Result<ImportReport> {
    let importer = importer(agent).with_context(|| {
        format!(
            "unknown importer '{agent}'; choose one of {}",
            IMPORTER_IDS.join(", ")
        )
    })?;
    importer.import(store, context)
}

pub fn import_all(store: &Store, context: &ImportContext) -> Result<Vec<ImportReport>> {
    all_importers()
        .into_iter()
        .map(|importer| importer.import(store, context))
        .collect()
}

pub(crate) fn empty_report(importer: &dyn Importer, paths: Vec<PathBuf>) -> ImportReport {
    ImportReport {
        agent: importer.id().to_string(),
        files_found: paths.len(),
        sources: paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        ..ImportReport::default()
    }
}

pub(crate) fn merge_file(report: &mut ImportReport, path: &Path, stats: &FileStats) {
    report.files_scanned += 1;
    report.events_seen += stats.events_seen;
    report.events_imported += stats.events_imported;
    report.duplicates += stats.duplicates;
    report.parse_errors += stats.parse_errors;
    merge_timestamp(&mut report.last_event_at, stats.last_event_at.as_deref());
    if !report
        .sources
        .iter()
        .any(|source| source == &path.display().to_string())
    {
        report.sources.push(path.display().to_string());
    }
}

pub(crate) fn record_source(
    store: &Store,
    agent: &str,
    path: &Path,
    stats: &FileStats,
) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64);
    store.record_import_source(
        agent,
        &path.display().to_string(),
        stats.next_offset,
        metadata.len().min(i64::MAX as u64) as i64,
        mtime,
        stats.events_imported,
        stats.parse_errors,
        stats.last_event_at.as_deref(),
    )
}

/// Read only complete newline-delimited records after a stored byte offset.
/// An incomplete final line stays unconsumed and is retried on the next pass.
pub(crate) fn read_complete_lines(path: &Path, offset: i64) -> Result<(Vec<(i64, String)>, i64)> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let start = if offset > 0 && offset as usize <= bytes.len() {
        offset as usize
    } else {
        0
    };
    let mut cursor = start;
    let mut lines = Vec::new();
    for segment in bytes[start..].split_inclusive(|byte| *byte == b'\n') {
        if !segment.ends_with(b"\n") {
            break;
        }
        let line_offset = cursor as i64;
        cursor += segment.len();
        let line = String::from_utf8_lossy(&segment[..segment.len() - 1])
            .trim_end_matches('\r')
            .to_string();
        lines.push((line_offset, line));
    }
    Ok((lines, cursor as i64))
}

pub(crate) fn cursor_for(store: &Store, path: &Path) -> Result<i64> {
    Ok(store
        .import_source(&path.display().to_string())?
        .map(|source| source.byte_offset)
        .unwrap_or(0))
}

pub(crate) fn insert_event(
    store: &Store,
    event: NewUsageEvent,
    stats: &mut FileStats,
) -> Result<()> {
    stats.events_seen += 1;
    merge_timestamp(&mut stats.last_event_at, event.ts.as_deref());
    if store.insert_usage_event(&event)? {
        stats.events_imported += 1;
    } else {
        stats.duplicates += 1;
    }
    Ok(())
}

pub(crate) fn unknown_cost() -> String {
    "unknown".to_string()
}

pub(crate) fn cost_source(cost_usd: Option<f64>) -> String {
    if cost_usd.is_some() {
        "reported".to_string()
    } else {
        unknown_cost()
    }
}

pub(crate) fn json_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToOwned::to_owned)
}

pub(crate) fn json_bool(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

pub(crate) fn json_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
    })
}

pub(crate) fn json_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| value.as_f64().filter(|number| number.is_finite()))
}

pub(crate) fn timestamp(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        if DateTime::parse_from_rfc3339(text).is_ok() {
            return Some(text.to_string());
        }
        return None;
    }
    let millis = value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))?;
    DateTime::<Utc>::from_timestamp_millis(millis).map(|date| date.to_rfc3339())
}

pub(crate) fn merge_timestamp(target: &mut Option<String>, candidate: Option<&str>) {
    let Some(candidate) = candidate else { return };
    if target
        .as_deref()
        .map(|existing| candidate > existing)
        .unwrap_or(true)
    {
        *target = Some(candidate.to_string());
    }
}

pub(crate) fn source_ref(path: &Path, locator: &str) -> String {
    format!("{}#{locator}", path.display())
}

/// Walk a bounded local tree while skipping dependency/cache trees that can
/// contain millions of files and are not agent session stores.
pub(crate) fn walk_files(root: &Path, predicate: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk_files_inner(root, &predicate, &mut found, 0);
    found.sort();
    found.dedup();
    found
}

fn walk_files_inner(
    root: &Path,
    predicate: &impl Fn(&Path) -> bool,
    found: &mut Vec<PathBuf>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir()
            && matches!(
                name.as_ref(),
                ".git" | "node_modules" | "target" | ".cache" | ".npm" | "Library"
            )
        {
            continue;
        }
        if path.is_dir() {
            walk_files_inner(&path, predicate, found, depth + 1);
        } else if path.is_file() && predicate(&path) {
            found.push(path);
        }
    }
}
