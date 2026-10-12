use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::Value;
use trace_core::importers::{self, ImportContext, Importer};
use trace_core::{Store, UsageEvent};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("trace-{label}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("temporary importer directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/agents")
        .join(relative)
}

fn expected(relative: &str) -> Value {
    let bytes = fs::read(fixture(relative)).expect("expected fixture");
    serde_json::from_slice(&bytes).expect("expected fixture JSON")
}

fn copied_fixture(relative: &str, destination: &Path) -> PathBuf {
    fs::copy(fixture(relative), destination).expect("copy fixture");
    destination.to_path_buf()
}

fn store() -> Store {
    Store::open_in_memory().expect("in-memory Trace store")
}

fn events(store: &Store, agent: &str) -> Vec<UsageEvent> {
    store
        .list_usage_events(Some(agent))
        .expect("canonical usage events")
}

#[test]
fn importer_order_is_codex_opencode_cursor_aider() {
    assert_eq!(
        importers::IMPORTER_IDS,
        ["codex", "opencode", "cursor", "aider"]
    );
}

#[test]
fn codex_imports_usage_only_and_preserves_unavailable_cost() {
    let temp = TempDir::new("codex-import");
    let path = copied_fixture(
        "codex/basic.jsonl",
        &temp.path().join("rollout-basic.jsonl"),
    );
    let store = store();

    let report = importers::codex::import_path(&store, &path).expect("Codex import");
    let expected = expected("codex/expected.json");
    assert_eq!(report.files_scanned, expected["files_scanned"]);
    assert_eq!(report.events_seen, expected["events_seen"]);
    assert_eq!(report.events_imported, expected["events_imported"]);
    assert_eq!(report.parse_errors, expected["parse_errors"]);

    let rows = events(&store, "codex");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].input_tokens, Some(1200));
    assert_eq!(rows[0].cache_read_tokens, Some(300));
    assert_eq!(rows[0].cost_usd, None);
    assert_eq!(rows[0].cost_source, "unknown");
    assert!(rows[1].is_subagent);
    assert_eq!(rows[1].parent_message_id.as_deref(), Some("codex-turn-1"));
    assert!(!serde_json::to_string(&rows).unwrap().contains("redacted"));

    let second = importers::codex::import_path(&store, &path).expect("idempotent Codex import");
    assert_eq!(second.events_seen, 0);
    assert_eq!(events(&store, "codex").len(), 2);
}

#[test]
fn opencode_reads_sqlite_metadata_and_never_reads_parts() {
    let temp = TempDir::new("opencode-import");
    let db_path = temp.path().join(".local/share/opencode/opencode.db");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let fixture: Value = serde_json::from_slice(
        &fs::read(fixture("opencode/messages.json")).expect("OpenCode fixture"),
    )
    .expect("OpenCode fixture JSON");
    let connection = Connection::open(&db_path).expect("fixture OpenCode database");
    connection
        .execute_batch(
            "CREATE TABLE session (id TEXT PRIMARY KEY, parent_id TEXT);
             CREATE TABLE message (id TEXT, session_id TEXT, time_created INTEGER, data TEXT);
             CREATE TABLE part (id TEXT, message_id TEXT, session_id TEXT, data TEXT);",
        )
        .unwrap();
    for session in fixture["sessions"].as_array().unwrap() {
        connection
            .execute(
                "INSERT INTO session (id, parent_id) VALUES (?1, ?2)",
                rusqlite::params![
                    session["id"].as_str().unwrap(),
                    session["parent_id"].as_str()
                ],
            )
            .unwrap();
    }
    for message in fixture["messages"].as_array().unwrap() {
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    message["id"].as_str().unwrap(),
                    message["session_id"].as_str().unwrap(),
                    message["time_created"].as_i64().unwrap(),
                    serde_json::to_string(&message["data"]).unwrap(),
                ],
            )
            .unwrap();
    }
    connection
        .execute(
            "INSERT INTO part (id, message_id, session_id, data) VALUES ('part-1', 'opencode-message-1', 'opencode-session-1', ?1)",
            ["prompt content that must not be read"],
        )
        .unwrap();
    drop(connection);

    let store = store();
    let context = ImportContext::with_home(temp.path());
    let report = importers::opencode::OpenCodeImporter
        .import(&store, &context)
        .expect("OpenCode import");
    let expected = expected("opencode/expected.json");
    assert_eq!(report.events_seen, expected["events_seen"]);
    assert_eq!(report.events_imported, expected["events_imported"]);

    let rows = events(&store, "opencode");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cost_usd, Some(0.42));
    assert_eq!(rows[0].cost_source, "reported");
    assert_eq!(rows[0].reported_cost_usd, Some(0.42));
    assert!(rows[0].computed_cost_usd.is_some());
    assert_eq!(rows[0].cost_reconciliation, "mismatch");
    assert!(rows[0].pricing_version.is_some());
    assert_eq!(rows[1].cost_usd, None);
    assert!(rows[1].is_subagent);
    assert!(!serde_json::to_string(&rows)
        .unwrap()
        .contains("prompt content"));
}

#[test]
fn cursor_imports_optional_metadata_and_marks_missing_usage_unknown() {
    let temp = TempDir::new("cursor-import");
    let session_dir = temp.path().join("project/agent-transcripts/session-1");
    fs::create_dir_all(&session_dir).unwrap();
    let path = copied_fixture("cursor/basic.jsonl", &session_dir.join("transcript.jsonl"));
    let store = store();

    let report = importers::cursor::import_path(&store, &path).expect("Cursor import");
    let expected = expected("cursor/expected.json");
    assert_eq!(report.events_seen, expected["events_seen"]);
    assert_eq!(report.parse_errors, expected["parse_errors"]);

    let rows = events(&store, "cursor");
    assert_eq!(rows.len(), 2);
    let usage = rows
        .iter()
        .find(|row| row.message_id == "cursor-message-1")
        .expect("Cursor usage row");
    assert_eq!(usage.input_tokens, Some(700));
    assert_eq!(usage.output_tokens, Some(140));
    assert_eq!(usage.cost_source, "unknown");
    let subagent = rows
        .iter()
        .find(|row| row.message_id == "cursor-message-2")
        .expect("Cursor subagent row");
    assert_eq!(subagent.input_tokens, None);
    assert!(subagent.is_subagent);
}

#[test]
fn aider_imports_explicit_summary_lines_without_assigning_timezone() {
    let temp = TempDir::new("aider-import");
    let path = copied_fixture(
        "aider/basic.chat.history.md",
        &temp.path().join(".aider.chat.history.md"),
    );
    let store = store();

    let report = importers::aider::import_path(&store, &path).expect("Aider import");
    let expected = expected("aider/expected.json");
    assert_eq!(report.events_seen, expected["events_seen"]);
    assert_eq!(report.events_imported, expected["events_imported"]);

    let rows = events(&store, "aider");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].input_tokens, Some(1200));
    assert_eq!(rows[0].output_tokens, Some(300));
    assert_eq!(rows[0].cost_usd, Some(0.42));
    assert_eq!(rows[0].cost_source, "reported");
    assert_eq!(rows[0].ts, None);
    assert_eq!(rows[1].input_tokens, None);
    assert_eq!(rows[1].cost_source, "unknown");
    assert!(!serde_json::to_string(&rows).unwrap().contains("redacted"));
}
