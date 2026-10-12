//! OpenAI Codex CLI rollout importer.
//!
//! Codex persists local rollouts as JSONL below `$CODEX_HOME/sessions` (or
//! `~/.codex/sessions`). We consume only structural metadata and
//! `token_usage_record`; prompts, response text, tool arguments, and tool
//! output are never copied into Trace.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::db::Store;
use crate::models::NewUsageEvent;

use super::{
    cost_source, cursor_for, empty_report, insert_event, json_bool, json_f64, json_i64,
    json_string, merge_file, read_complete_lines, record_source, source_ref, timestamp, FileStats,
    ImportContext, ImportReport, Importer,
};

#[derive(Debug, Clone, Copy)]
pub struct CodexImporter;

impl Importer for CodexImporter {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex CLI"
    }

    fn discover(&self, context: &ImportContext) -> Vec<PathBuf> {
        let root = codex_home(context).join("sessions");
        super::walk_files(&root, |path| {
            path.extension().is_some_and(|ext| ext == "jsonl")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
        })
    }

    fn import(&self, store: &Store, context: &ImportContext) -> Result<ImportReport> {
        let paths = self.discover(context);
        let mut report = empty_report(self, paths.clone());
        for path in paths {
            let stats = import_file(store, &path)?;
            merge_file(&mut report, &path, &stats);
            record_source(store, self.id(), &path, &stats)?;
        }
        Ok(report)
    }
}

fn codex_home(context: &ImportContext) -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| context.home.join(".codex"))
}

fn session_id_from_file(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    let payload = value.get("payload")?.as_object()?;
    json_string(payload.get("id").or_else(|| payload.get("session_id")))
}

fn import_file(store: &Store, path: &Path) -> Result<FileStats> {
    let cursor = cursor_for(store, path)?;
    let (lines, next_offset) = read_complete_lines(path, cursor)?;
    let fallback_session = path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown-codex-session".to_string());
    let session_id = session_id_from_file(path).unwrap_or(fallback_session);
    let mut stats = FileStats {
        next_offset,
        ..FileStats::default()
    };

    for (offset, line) in lines {
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                stats.parse_errors += 1;
                continue;
            }
        };
        if value.get("type").and_then(Value::as_str) != Some("token_usage_record") {
            continue;
        }
        let Some(payload) = value.get("payload").and_then(Value::as_object) else {
            stats.parse_errors += 1;
            continue;
        };
        let Some(usage) = payload.get("usage").and_then(Value::as_object) else {
            stats.parse_errors += 1;
            continue;
        };

        let message_id = json_string(payload.get("response_id"))
            .or_else(|| json_string(payload.get("turn_id")))
            .unwrap_or_else(|| format!("offset:{offset}"));
        let event_session =
            json_string(payload.get("session_id")).unwrap_or_else(|| session_id.clone());
        let cost_usd = json_f64(payload.get("cost_usd").or_else(|| payload.get("cost")));
        let is_subagent = json_bool(payload.get("is_subagent")).unwrap_or_else(|| {
            payload
                .get("agent_type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.eq_ignore_ascii_case("subagent"))
        });
        let subagent_id = json_string(payload.get("subagent_id"))
            .or_else(|| json_string(payload.get("agent_id")));
        let parent_message_id = json_string(payload.get("parent_message_id"))
            .or_else(|| json_string(payload.get("parent_turn_id")))
            .or_else(|| json_string(payload.get("parent_id")));
        let ts = timestamp(value.get("timestamp").or_else(|| payload.get("timestamp")));
        let model = json_string(payload.get("model"))
            .or_else(|| json_string(payload.get("model_id")))
            .or_else(|| json_string(payload.get("model_name")));

        insert_event(
            store,
            NewUsageEvent {
                run_id: None,
                agent: "codex".to_string(),
                session_id: event_session,
                message_id,
                parent_message_id,
                is_subagent,
                subagent_id,
                model,
                input_tokens: json_i64(usage.get("input_tokens")),
                output_tokens: json_i64(usage.get("output_tokens")),
                cache_read_tokens: json_i64(usage.get("cached_input_tokens")),
                cache_write_tokens: json_i64(usage.get("cache_write_input_tokens")),
                cache_write_5m_tokens: None,
                cache_write_1h_tokens: None,
                reasoning_tokens: json_i64(usage.get("reasoning_output_tokens")),
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

/// Kept as a small public helper for fixture tests and future callers that
/// already have a single rollout path.
pub fn import_path(store: &Store, path: &Path) -> Result<ImportReport> {
    let importer = CodexImporter;
    let mut report = empty_report(&importer, vec![path.to_path_buf()]);
    let stats =
        import_file(store, path).with_context(|| format!("importing {}", path.display()))?;
    merge_file(&mut report, path, &stats);
    record_source(store, importer.id(), path, &stats)?;
    Ok(report)
}
