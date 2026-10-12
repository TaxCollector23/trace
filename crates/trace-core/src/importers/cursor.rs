//! Cursor local transcript importer.
//!
//! Cursor documents its native chat history as local SQLite, but does not
//! publish a stable schema for the opaque conversation blobs. On this machine
//! the inspectable agent surface is `projects/**/agent-transcripts/*.jsonl`,
//! which this importer consumes. Native chat databases are still reported as
//! found by doctor, but are not decoded speculatively.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::Store;
use crate::models::NewUsageEvent;

use super::{
    cost_source, cursor_for, empty_report, insert_event, json_bool, json_f64, json_i64,
    json_string, merge_file, record_source, source_ref, timestamp, FileStats, ImportContext,
    ImportReport, Importer,
};

#[derive(Debug, Clone, Copy)]
pub struct CursorImporter;

impl Importer for CursorImporter {
    fn id(&self) -> &'static str {
        "cursor"
    }

    fn display_name(&self) -> &'static str {
        "Cursor"
    }

    fn discover(&self, context: &ImportContext) -> Vec<PathBuf> {
        let root = context.home.join(".cursor");
        super::walk_files(&root, |path| {
            let transcript = has_component(path, "agent-transcripts")
                && path.extension().is_some_and(|ext| ext == "jsonl");
            let native_store = path.file_name().is_some_and(|name| name == "store.db");
            transcript || native_store
        })
    }

    fn import(&self, store: &Store, context: &ImportContext) -> Result<ImportReport> {
        let paths = self.discover(context);
        let mut report = empty_report(self, paths.clone());
        for path in paths {
            let is_transcript = has_component(&path, "agent-transcripts")
                && path.extension().is_some_and(|ext| ext == "jsonl");
            let stats = if is_transcript {
                import_transcript(store, &path)?
            } else {
                // The native store is intentionally metadata-only until Cursor
                // documents the blob schema. Finding it is useful diagnostics;
                // treating opaque blobs as usage would be fabricated data.
                FileStats {
                    next_offset: fs::metadata(&path)
                        .map(|metadata| metadata.len().min(i64::MAX as u64) as i64)
                        .unwrap_or(0),
                    ..FileStats::default()
                }
            };
            merge_file(&mut report, &path, &stats);
            record_source(store, self.id(), &path, &stats)?;
        }
        Ok(report)
    }
}

fn has_component(path: &Path, expected: &str) -> bool {
    path.components()
        .any(|component| component.as_os_str() == expected)
}

fn session_id(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .map(|name| format!("cursor:{}", name.to_string_lossy()))
        .unwrap_or_else(|| "cursor:unknown-session".to_string())
}

fn import_transcript(store: &Store, path: &Path) -> Result<FileStats> {
    let cursor = cursor_for(store, path)?;
    let (lines, next_offset) = super::read_complete_lines(path, cursor)
        .with_context(|| format!("reading Cursor transcript {}", path.display()))?;
    let mut stats = FileStats {
        next_offset,
        ..FileStats::default()
    };
    let session_id = session_id(path);

    for (offset, line) in lines {
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                stats.parse_errors += 1;
                continue;
            }
        };
        let payload = value.get("message").unwrap_or(&value);
        if payload.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let usage = payload
            .get("usage")
            .or_else(|| payload.get("tokens"))
            .and_then(Value::as_object);
        let cache = usage
            .and_then(|usage| usage.get("cache"))
            .and_then(Value::as_object);
        let cost_usd = json_f64(payload.get("cost_usd").or_else(|| payload.get("cost")));
        let is_subagent = json_bool(payload.get("is_subagent")).unwrap_or_else(|| {
            payload
                .get("agent_type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.eq_ignore_ascii_case("subagent"))
        });
        let message_id = json_string(payload.get("id"))
            .or_else(|| json_string(value.get("id")))
            .unwrap_or_else(|| format!("offset:{offset}"));
        let ts = timestamp(
            payload
                .get("timestamp")
                .or_else(|| payload.get("createdAt"))
                .or_else(|| value.get("timestamp"))
                .or_else(|| value.get("createdAt")),
        );

        insert_event(
            store,
            NewUsageEvent {
                run_id: None,
                agent: "cursor".to_string(),
                session_id: session_id.clone(),
                message_id,
                parent_message_id: json_string(payload.get("parentID"))
                    .or_else(|| json_string(payload.get("parent_message_id"))),
                is_subagent,
                subagent_id: json_string(payload.get("subagent_id")),
                model: json_string(payload.get("model"))
                    .or_else(|| json_string(payload.get("modelID"))),
                input_tokens: usage.and_then(|usage| {
                    json_i64(usage.get("input_tokens").or_else(|| usage.get("input")))
                }),
                output_tokens: usage.and_then(|usage| {
                    json_i64(usage.get("output_tokens").or_else(|| usage.get("output")))
                }),
                cache_read_tokens: cache.and_then(|cache| json_i64(cache.get("read"))),
                cache_write_tokens: cache.and_then(|cache| json_i64(cache.get("write"))),
                cache_write_5m_tokens: None,
                cache_write_1h_tokens: None,
                reasoning_tokens: usage.and_then(|usage| json_i64(usage.get("reasoning_tokens"))),
                ts,
                cost_usd,
                cost_source: cost_source(cost_usd),
                raw_ref: source_ref(path, &format!("byte={offset}")),
            },
            &mut stats,
        )?;
    }
    Ok(stats)
}

pub fn import_path(store: &Store, path: &Path) -> Result<ImportReport> {
    let importer = CursorImporter;
    let mut report = empty_report(&importer, vec![path.to_path_buf()]);
    let stats = import_transcript(store, path)
        .with_context(|| format!("importing Cursor transcript {}", path.display()))?;
    merge_file(&mut report, path, &stats);
    record_source(store, importer.id(), path, &stats)?;
    Ok(report)
}
