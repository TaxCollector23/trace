//! OpenCode local SQLite importer.
//!
//! Current OpenCode stores sessions, messages, and tool parts in SQLite. The
//! importer reads only `session` and assistant `message` metadata. It never
//! reads the text-bearing `part.data` fields, and a reported cost of `0` is
//! preserved as reported rather than changed into unavailable.

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::db::Store;
use crate::models::NewUsageEvent;

use super::{
    cost_source, empty_report, insert_event, json_f64, json_i64, json_string, merge_file,
    record_source, source_ref, timestamp, FileStats, ImportContext, ImportReport, Importer,
};

#[derive(Debug, Clone, Copy)]
pub struct OpenCodeImporter;

impl Importer for OpenCodeImporter {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
    }

    fn discover(&self, context: &ImportContext) -> Vec<PathBuf> {
        let root = data_root(context);
        super::walk_files(&root, |path| {
            path.file_name().is_some_and(|name| name == "opencode.db")
                || (path.extension().is_some_and(|ext| ext == "db")
                    && path
                        .parent()
                        .and_then(Path::file_name)
                        .is_some_and(|name| name == "storage"))
        })
    }

    fn import(&self, store: &Store, context: &ImportContext) -> Result<ImportReport> {
        let paths = self.discover(context);
        let mut report = empty_report(self, paths.clone());
        for path in paths {
            let stats = import_db(store, &path)
                .with_context(|| format!("importing OpenCode database {}", path.display()))?;
            merge_file(&mut report, &path, &stats);
            record_source(store, self.id(), &path, &stats)?;
        }
        Ok(report)
    }
}

fn data_root(context: &ImportContext) -> PathBuf {
    if let Some(path) = std::env::var_os("OPENCODE_DATA_DIR") {
        return PathBuf::from(path);
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("opencode");
    }
    context.home.join(".local").join("share").join("opencode")
}

fn import_db(store: &Store, path: &Path) -> Result<FileStats> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening {} read-only", path.display()))?;
    let mut statement = connection.prepare(
        "SELECT m.rowid, m.id, m.session_id, m.time_created, m.data,
                s.parent_id
         FROM message AS m
         LEFT JOIN session AS s ON s.id = m.session_id
         ORDER BY m.time_created ASC, m.rowid ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut stats = FileStats {
        next_offset: std::fs::metadata(path)
            .map(|metadata| metadata.len().min(i64::MAX as u64) as i64)
            .unwrap_or(0),
        ..FileStats::default()
    };

    for row in rows {
        let (rowid, message_id, session_id, time_created, data, parent_session_id) = row?;
        let value: Value = match serde_json::from_str(&data) {
            Ok(value) => value,
            Err(_) => {
                stats.parse_errors += 1;
                continue;
            }
        };
        if value.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }

        let tokens = value.get("tokens").and_then(Value::as_object);
        let cache = tokens
            .and_then(|tokens| tokens.get("cache"))
            .and_then(Value::as_object);
        let cost_usd = json_f64(value.get("cost"));
        let model = json_string(value.get("modelID"));
        let parent_message_id = json_string(value.get("parentID"));
        let is_subagent = parent_session_id.is_some();
        let subagent_id = parent_session_id.as_ref().map(|_| session_id.clone());
        let time_value = Value::from(time_created);

        insert_event(
            store,
            NewUsageEvent {
                run_id: None,
                agent: "opencode".to_string(),
                session_id,
                message_id,
                parent_message_id,
                is_subagent,
                subagent_id,
                model,
                input_tokens: tokens.and_then(|tokens| json_i64(tokens.get("input"))),
                output_tokens: tokens.and_then(|tokens| json_i64(tokens.get("output"))),
                cache_read_tokens: cache.and_then(|cache| json_i64(cache.get("read"))),
                cache_write_tokens: cache.and_then(|cache| json_i64(cache.get("write"))),
                cache_write_5m_tokens: None,
                cache_write_1h_tokens: None,
                reasoning_tokens: tokens.and_then(|tokens| json_i64(tokens.get("reasoning"))),
                ts: timestamp(Some(&time_value)),
                cost_usd,
                cost_source: cost_source(cost_usd),
                raw_ref: source_ref(path, &format!("rowid={rowid}")),
            },
            &mut stats,
        )?;
    }
    Ok(stats)
}

pub fn import_path(store: &Store, path: &Path) -> Result<ImportReport> {
    let importer = OpenCodeImporter;
    let mut report = empty_report(&importer, vec![path.to_path_buf()]);
    let stats = import_db(store, path).with_context(|| format!("importing {}", path.display()))?;
    merge_file(&mut report, path, &stats);
    record_source(store, importer.id(), path, &stats)?;
    Ok(report)
}
