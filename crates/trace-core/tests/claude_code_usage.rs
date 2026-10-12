use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use trace_core::{
    usage::{ClaudeCodeImporter, UsageGroup, UsageImporter},
    Store,
};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/claude-code")
        .canonicalize()
        .expect("Claude Code fixture directory")
}

#[test]
fn imports_metadata_dedupes_streaming_rows_and_attributes_sidechains() {
    let store = Store::open_in_memory().expect("in-memory store");
    let importer = ClaudeCodeImporter::new(fixture_root());

    let first = importer.import(&store).expect("first import");
    assert_eq!(first.agent, "claude-code");
    assert_eq!(first.files_seen, 2);
    assert_eq!(first.lines_read, 8);
    assert_eq!(first.events_inserted, 4);
    assert_eq!(first.events_updated, 1);
    assert_eq!(first.malformed_lines, 1);
    assert_eq!(first.last_event_at.as_deref(), Some("2026-01-15T10:00:06Z"));

    let events = store
        .list_usage_events(Some("claude-code"))
        .expect("canonical usage events");
    assert_eq!(events.len(), 4);

    let streamed = events
        .iter()
        .find(|event| event.message_id == "msg-stream-1")
        .expect("streaming event");
    assert_eq!(streamed.output_tokens, Some(9));

    let child = events
        .iter()
        .find(|event| event.message_id == "msg-child-1")
        .expect("sidechain event");
    assert!(child.is_subagent);
    assert_eq!(child.subagent_id.as_deref(), Some("child-01"));
    assert_eq!(child.parent_message_id.as_deref(), Some("toolu-parent-1"));

    let unavailable = events
        .iter()
        .find(|event| event.message_id == "msg-no-model")
        .expect("missing-model event");
    assert_eq!(unavailable.model, None);
    assert_eq!(unavailable.cost_usd, None);
    assert_eq!(unavailable.cost_source, "unknown");

    let serialized_events = serde_json::to_string(&events).expect("serialize events");
    assert!(!serialized_events.contains("fixture prompt that must not persist"));
    assert!(!serialized_events.contains("fixture response that must not persist"));

    let response = store
        .usage_response(UsageGroup::Model, None, first.clone())
        .expect("usage response");
    assert_eq!(response.totals.event_count, 4);
    assert_eq!(response.totals.input_tokens, Some(241));
    assert_eq!(response.totals.output_tokens, Some(43));
    assert_eq!(response.totals.cache_read_tokens, Some(30));
    assert_eq!(response.totals.cache_write_tokens, Some(40));
    assert_eq!(response.totals.reasoning_tokens, Some(5));
    assert_eq!(response.totals.cost_usd, None);
    assert_eq!(response.totals.unavailable_cost_events, 4);
    assert_eq!(response.totals.subagent_events, 1);
    assert!(serde_json::to_string(&response)
        .expect("serialize response")
        .contains("\"cost_usd\":null"));

    let second = importer.import(&store).expect("incremental import");
    assert_eq!(second.lines_read, 0);
    assert_eq!(second.events_inserted, 0);
    assert_eq!(second.events_updated, 0);
    assert_eq!(second.malformed_lines, 0);
    assert_eq!(
        store
            .list_usage_events(Some("claude-code"))
            .expect("events after second import")
            .len(),
        4
    );

    let sources = store
        .import_sources(Some("claude-code"))
        .expect("import source cursors");
    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .all(|source| source.byte_offset == source.file_size));
    assert_eq!(
        sources
            .iter()
            .map(|source| source.parse_errors)
            .sum::<i64>(),
        1
    );
}

#[test]
fn empty_usage_has_unavailable_totals_instead_of_sql_null_errors() {
    let store = Store::open_in_memory().expect("in-memory store");
    let response = store
        .usage_response(
            UsageGroup::Model,
            None,
            trace_core::usage::ImportReport {
                agent: "claude-code".to_string(),
                ..Default::default()
            },
        )
        .expect("empty usage response");
    assert_eq!(response.totals.event_count, 0);
    assert_eq!(response.totals.cost_usd, None);
    assert_eq!(response.totals.unavailable_cost_events, 0);
    assert_eq!(response.totals.subagent_events, 0);
}

#[test]
fn retries_an_unterminated_final_record_after_newline_arrives() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("trace-claude-partial-{suffix}"));
    let project = root.join("project");
    fs::create_dir_all(&project).expect("temporary project directory");
    let path = project.join("session.jsonl");
    let source = fixture_root().join("project-alpha/session-main.jsonl");
    let transcript = fs::read_to_string(source).expect("fixture transcript");
    let assistant_line = transcript.lines().nth(1).expect("assistant fixture line");
    fs::write(&path, assistant_line).expect("partial transcript");

    let store = Store::open_in_memory().expect("in-memory store");
    let importer = ClaudeCodeImporter::new(&root);
    let first = importer.import(&store).expect("partial import");
    assert_eq!(first.lines_read, 0);
    assert_eq!(first.events_inserted, 0);
    assert_eq!(
        store
            .import_source(&path.display().to_string())
            .expect("partial source")
            .expect("stored cursor")
            .byte_offset,
        0
    );

    let mut append = OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open partial transcript");
    append.write_all(b"\n").expect("complete transcript line");
    let second = importer.import(&store).expect("completed import");
    assert_eq!(second.lines_read, 1);
    assert_eq!(second.events_inserted, 1);
    assert_eq!(
        store
            .list_usage_events(Some("claude-code"))
            .expect("completed events")
            .len(),
        1
    );

    fs::remove_dir_all(root).expect("remove temporary fixture");
}
