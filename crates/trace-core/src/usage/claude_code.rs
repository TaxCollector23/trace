//! Claude Code transcript importer.
//!
//! Claude Code's local JSONL transcript is an internal format. This importer
//! intentionally extracts only assistant usage metadata and treats every
//! other record as non-usage input. It never reads or stores content blocks.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::usage::{ImportReport, ImportSourceStatus, NewUsageEvent};
use crate::{Store, UsageImporter};

const AGENT: &str = "claude-code";

/// Reads Claude Code's local project transcript tree.
#[derive(Debug, Clone)]
pub struct ClaudeCodeImporter {
    projects_dir: PathBuf,
}

impl ClaudeCodeImporter {
    pub fn new(projects_dir: impl Into<PathBuf>) -> Self {
        Self {
            projects_dir: projects_dir.into(),
        }
    }

    /// Resolve the normal Claude Code transcript root. `CLAUDE_CONFIG_DIR` is
    /// honored because the official CLI supports moving its local storage.
    pub fn default_projects_dir() -> Result<PathBuf> {
        let config_dir = std::env::var_os("CLAUDE_CONFIG_DIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".claude")))
            .context("could not determine the Claude Code config directory")?;
        Ok(config_dir.join("projects"))
    }

    pub fn from_default_location() -> Result<Self> {
        Ok(Self::new(Self::default_projects_dir()?))
    }

    pub fn projects_dir(&self) -> &Path {
        &self.projects_dir
    }

    fn discover_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if !self.projects_dir.exists() {
            return Ok(files);
        }
        discover_recursive(&self.projects_dir, &mut files)?;
        files.sort();
        Ok(files)
    }
}

impl UsageImporter for ClaudeCodeImporter {
    fn agent(&self) -> &'static str {
        AGENT
    }

    fn import(&self, store: &Store) -> Result<ImportReport> {
        let mut report = ImportReport {
            agent: AGENT.to_string(),
            ..ImportReport::default()
        };

        for path in self.discover_files()? {
            let file_report = import_file(store, &path, &self.projects_dir)?;
            report.merge(&file_report);
        }
        Ok(report)
    }
}

fn discover_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            discover_recursive(&path, files)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }
    Ok(())
}

fn import_file(store: &Store, path: &Path, projects_dir: &Path) -> Result<ImportReport> {
    let path_string = path.to_string_lossy().into_owned();
    let previous = store
        .import_source(&path_string)?
        .unwrap_or_else(|| ImportSourceStatus {
            agent: AGENT.to_string(),
            path: path_string.clone(),
            byte_offset: 0,
            file_size: 0,
            file_mtime: None,
            events_imported: 0,
            parse_errors: 0,
            last_event_at: None,
        });
    let file_size = fs::metadata(path)?.len();
    let start_offset = if previous.byte_offset < 0 || file_size < previous.byte_offset as u64 {
        0
    } else {
        previous.byte_offset as u64
    };

    let file =
        File::open(path).with_context(|| format!("opening Claude Code log {}", path.display()))?;
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start_offset))?;
    let mut offset = start_offset;
    let mut report = ImportReport {
        agent: AGENT.to_string(),
        files_seen: 1,
        ..ImportReport::default()
    };
    let mut line = Vec::new();

    loop {
        line.clear();
        let line_start = offset;
        let bytes = reader.read_until(b'\n', &mut line)?;
        if bytes == 0 {
            break;
        }
        let complete_line = line.last() == Some(&b'\n');
        if !complete_line {
            // Claude Code appends JSONL records while a process is alive. Do
            // not count or advance past a partial final record; the next
            // import will retry it after the newline arrives.
            break;
        }

        offset += bytes as u64;
        report.lines_read += 1;
        match parse_line(&line, path, projects_dir, line_start) {
            Ok(Some(event)) => record_event(store, &event, &mut report)?,
            Ok(None) => {}
            Err(_) => report.malformed_lines += 1,
        }
    }

    let file_mtime = fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_secs()).ok());
    let new_parse_errors = report.malformed_lines;
    store.record_import_source(
        AGENT,
        &path_string,
        i64::try_from(offset).context("Claude Code log offset exceeds SQLite range")?,
        i64::try_from(file_size).context("Claude Code log size exceeds SQLite range")?,
        file_mtime,
        report.events_inserted,
        new_parse_errors,
        report.last_event_at.as_deref(),
    )?;
    Ok(report)
}

fn record_event(store: &Store, event: &NewUsageEvent, report: &mut ImportReport) -> Result<()> {
    if let Some(ts) = event.ts.as_deref() {
        let replace = report
            .last_event_at
            .as_deref()
            .map_or(true, |current| ts > current);
        if replace {
            report.last_event_at = Some(ts.to_string());
        }
    }
    match store.upsert_usage_event(event)? {
        crate::usage::UsageUpsertResult::Inserted => report.events_inserted += 1,
        crate::usage::UsageUpsertResult::Updated => report.events_updated += 1,
    }
    Ok(())
}

fn parse_line(
    line: &[u8],
    path: &Path,
    projects_dir: &Path,
    offset: u64,
) -> std::result::Result<Option<NewUsageEvent>, ParseLineError> {
    let value: Value = serde_json::from_slice(line).map_err(|_| ParseLineError)?;
    let object = value.as_object().ok_or(ParseLineError)?;
    if object.get("type").and_then(Value::as_str) != Some("assistant") {
        return Ok(None);
    }
    let Some(message) = object.get("message").and_then(Value::as_object) else {
        return Ok(None);
    };
    let Some(usage) = message.get("usage").and_then(Value::as_object) else {
        return Ok(None);
    };

    let session_id = object
        .get("sessionId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| file_session_id(path, projects_dir))
        .ok_or(ParseLineError)?;
    let message_id = message
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(ParseLineError)?
        .to_string();
    let ts = object
        .get("timestamp")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(ParseLineError)?
        .to_string();

    let input_tokens = integer_field(usage, "input_tokens")?;
    let output_tokens = integer_field(usage, "output_tokens")?;
    let cache_read_tokens = integer_field(usage, "cache_read_input_tokens")?;
    let cache_write_tokens = integer_field(usage, "cache_creation_input_tokens")?;
    let cache_write_5m_tokens =
        nested_integer_field(usage, "cache_creation", "ephemeral_5m_input_tokens");
    let cache_write_1h_tokens =
        nested_integer_field(usage, "cache_creation", "ephemeral_1h_input_tokens");
    let reasoning_tokens = integer_field(usage, "reasoning_tokens")?
        .or_else(|| nested_integer_field(usage, "output_tokens_details", "reasoning_tokens"))
        .or_else(|| nested_integer_field(usage, "output_tokens_details", "thinking_tokens"));

    let subagent_id = object
        .get("agentId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| path_subagent_id(path, projects_dir));
    let is_subagent = object
        .get("isSidechain")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || subagent_id.is_some();
    let parent_message_id = if is_subagent {
        subagent_parent_message_id(path).or_else(|| string_field(object, "parentUuid"))
    } else {
        string_field(object, "parentUuid")
    };

    let (cost_usd, cost_source) = reported_cost(object, message, usage);
    Ok(Some(NewUsageEvent {
        run_id: None,
        agent: AGENT.to_string(),
        session_id,
        message_id,
        parent_message_id,
        is_subagent,
        subagent_id,
        model: message
            .get("model")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        cache_write_5m_tokens,
        cache_write_1h_tokens,
        reasoning_tokens,
        ts: Some(ts),
        cost_usd,
        cost_source,
        raw_ref: format!("{}#{offset}", path.display()),
    }))
}

#[derive(Debug, Clone, Copy)]
struct ParseLineError;

fn integer_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
) -> std::result::Result<Option<i64>, ParseLineError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value.as_i64().ok_or(ParseLineError).map(Some),
        Some(_) => Err(ParseLineError),
    }
}

fn nested_integer_field(
    object: &serde_json::Map<String, Value>,
    parent: &str,
    name: &str,
) -> Option<i64> {
    object
        .get(parent)
        .and_then(Value::as_object)
        .and_then(|nested| nested.get(name))
        .and_then(Value::as_i64)
}

fn string_field(object: &serde_json::Map<String, Value>, name: &str) -> Option<String> {
    object
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn reported_cost(
    object: &serde_json::Map<String, Value>,
    message: &serde_json::Map<String, Value>,
    usage: &serde_json::Map<String, Value>,
) -> (Option<f64>, String) {
    [object, message, usage]
        .into_iter()
        .find_map(|candidate| {
            ["cost_usd", "cost"]
                .into_iter()
                .find_map(|key| candidate.get(key).and_then(Value::as_f64))
        })
        .map_or((None, "unknown".to_string()), |cost| {
            (Some(cost), "reported".to_string())
        })
}

fn file_session_id(path: &Path, projects_dir: &Path) -> Option<String> {
    let relative = path.strip_prefix(projects_dir).ok()?;
    let mut components = relative.components();
    let _project = components.next()?;
    let session = components.next()?;
    if components.next().is_none() && path.extension().is_some_and(|ext| ext == "jsonl") {
        return session
            .as_os_str()
            .to_str()?
            .strip_suffix(".jsonl")
            .map(ToOwned::to_owned);
    }
    if session.as_os_str() == "subagents" {
        return None;
    }
    session.as_os_str().to_str().map(ToOwned::to_owned)
}

fn path_subagent_id(path: &Path, projects_dir: &Path) -> Option<String> {
    let relative = path.strip_prefix(projects_dir).ok()?;
    let components: Vec<_> = relative.components().collect();
    let subagents_index = components
        .iter()
        .position(|component| component.as_os_str() == "subagents")?;
    let file = components.get(subagents_index + 1)?.as_os_str().to_str()?;
    let stem = file.strip_suffix(".jsonl")?;
    Some(stem.strip_prefix("agent-").unwrap_or(stem).to_string())
}

fn subagent_parent_message_id(path: &Path) -> Option<String> {
    let meta = path.with_extension("meta.json");
    let raw = fs::read_to_string(meta).ok()?;
    serde_json::from_str::<Value>(&raw)
        .ok()?
        .get("toolUseId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_subagent_path_fallbacks_are_conservative() {
        let root = Path::new("/tmp/claude/projects");
        let main = root.join("project/session.jsonl");
        assert_eq!(file_session_id(&main, root).as_deref(), Some("session"));
        let child = root.join("project/session/subagents/agent-child.jsonl");
        assert_eq!(path_subagent_id(&child, root).as_deref(), Some("child"));
    }
}
