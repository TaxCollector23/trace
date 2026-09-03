//! Maps the *current* stored shape (`events` + `commands` rows) into
//! [`NormalizedEvent`]s.
//!
//! This is the single seam between "whatever Trace happens to persist today"
//! and the stable `NormalizedEvent` contract the dashboard and every analyzer
//! read. If the schema changes, only this file (and `status_and_risk_for_event_kind`)
//! needs to change.

use serde_json::Value;

use crate::intel::event::{EventStatus, NormalizedEvent, RiskLevel};
use crate::models::{CommandRecord, Event, Run};

/// Deterministic status/risk lookup for a stored `events.type` value (the
/// `EventType::as_str()` vocabulary in `models.rs`). An event kind with no
/// entry here is genuinely uncharacterized: it maps to `(Unknown, None)`
/// rather than a guess.
fn status_and_risk_for_event_kind(kind: &str) -> (EventStatus, RiskLevel) {
    use EventStatus::*;
    use RiskLevel::*;
    match kind {
        "run_created" | "session_started" => (Started, None),
        "prompt_submitted" | "command_started" | "tool_call_started" | "build_started"
        | "tests_started" | "agent_thinking" => (Running, None),
        "prompt_finished"
        | "tool_call_finished"
        | "command_output"
        | "checkpoint_created"
        | "file_created"
        | "file_modified"
        | "file_opened"
        | "directory_created"
        | "api_usage_recorded"
        | "build_passed"
        | "tests_passed"
        | "final_diff_captured"
        | "run_completed"
        | "note"
        | "session_ended"
        | "git_status_changed"
        | "commit_detected"
        | "branch_changed"
        | "agent_idle"
        | "replay_marker" => (Ok, None),
        // A deletion is irreversible without a checkpoint; worth a low-risk tag
        // even though it is not itself blocked or warned.
        "file_deleted" | "directory_deleted" => (Ok, Low),
        "risky_command_warned" => (Warn, Medium),
        "risky_command_blocked" => (Blocked, High),
        // The guard required approval and it was granted — an elevated-risk
        // action still went ahead, which is worth surfacing even though it
        // completed normally.
        "command_approved" => (Ok, Medium),
        "secret_warning" => (Warn, High),
        "build_failed" | "tests_failed" | "run_failed" => (Failed, Low),
        "rollback_created" => (Running, Medium),
        "rollback_completed" => (Ok, Medium),
        "risk_detected" => (Warn, Medium),
        _ => (Unknown, None),
    }
}

/// Deterministic status/risk lookup for a stored `commands.decision` value
/// (`guard::Decision::as_str()`), combined with the command's exit code once
/// known. `None` exit code (still running, or never captured) never becomes a
/// fabricated `Ok`/`Failed` — it stays `Running`/`Unknown` as appropriate.
fn status_and_risk_for_command(decision: &str, exit_code: Option<i64>) -> (EventStatus, RiskLevel) {
    use EventStatus::*;
    use RiskLevel::*;
    match decision {
        "block" | "blocked" => (Blocked, High),
        "require_approval" => (PendingApproval, Medium),
        "warn" => (Warn, Medium),
        "allow" => match exit_code {
            Some(0) => (Ok, None),
            Some(_) => (Failed, Low),
            Option::None => (Running, None),
        },
        _ => (Unknown, None),
    }
}

/// Best-effort `target` extraction from a stored `metadata_json` blob: the
/// first of `path`, `file_path`, `git_ref`, `command` that is present as a
/// string. Returns `None` when nothing usable is found — never fabricated.
fn extract_target(metadata: &Option<Value>) -> Option<String> {
    let obj = metadata.as_ref()?.as_object()?;
    for key in ["path", "file_path", "git_ref", "command"] {
        if let Some(s) = obj.get(key).and_then(Value::as_str) {
            return Some(s.to_string());
        }
    }
    None
}

fn parse_metadata(metadata_json: &Option<String>) -> Option<Value> {
    let raw = metadata_json.as_ref()?;
    let v: Value = serde_json::from_str(raw).ok()?;
    // Only an object counts as "metadata" per the NormalizedEvent contract
    // (`Record<string, unknown> | null`); anything else parses but is not a
    // metadata object, so it is dropped rather than misrepresented.
    if v.is_object() {
        Some(v)
    } else {
        None
    }
}

fn actor_for(run: &Run) -> String {
    run.agent_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// One normalized event for a stored `events` row.
fn normalize_event(run: &Run, e: &Event) -> NormalizedEvent {
    let metadata = parse_metadata(&e.metadata_json);
    let (status, risk) = status_and_risk_for_event_kind(&e.event_type);
    let target = extract_target(&metadata);
    let evidence = serde_json::json!({
        "message": e.message,
        "metadata": metadata,
    });
    NormalizedEvent {
        id: e.id.clone(),
        run_id: e.run_id.clone(),
        parent_id: None,
        ts_start: e.created_at.clone(),
        ts_end: None,
        kind: e.event_type.clone(),
        actor: actor_for(run),
        source: "trace".to_string(),
        status: status.as_str().to_string(),
        risk: risk.as_str().to_string(),
        target,
        evidence,
        metadata,
    }
}

/// One normalized event for a stored `commands` row. Commands are surfaced
/// under the synthetic kind `"command_executed"` since the DB has no separate
/// "command finished" event distinct from the row itself.
fn normalize_command(run: &Run, c: &CommandRecord) -> NormalizedEvent {
    let (status, risk) = status_and_risk_for_command(&c.decision, c.exit_code);
    let evidence = serde_json::json!({
        "command": c.command,
        "decision": c.decision,
        "exit_code": c.exit_code,
    });
    NormalizedEvent {
        id: c.id.clone(),
        run_id: c.run_id.clone(),
        parent_id: None,
        ts_start: c.created_at.clone(),
        ts_end: None,
        kind: "command_executed".to_string(),
        actor: actor_for(run),
        source: "trace".to_string(),
        status: status.as_str().to_string(),
        risk: risk.as_str().to_string(),
        target: Some(c.command.clone()),
        evidence,
        metadata: None,
    }
}

/// Normalize every recorded event and command for a run into one
/// chronologically-ordered `NormalizedEvent` timeline. Stored timestamps are
/// RFC 3339 strings, which sort lexicographically in time order; ties (same
/// second) keep stored order via a stable sort.
pub fn normalize_run(
    run: &Run,
    events: &[Event],
    commands: &[CommandRecord],
) -> Vec<NormalizedEvent> {
    let mut out: Vec<NormalizedEvent> = Vec::with_capacity(events.len() + commands.len());
    out.extend(events.iter().map(|e| normalize_event(run, e)));
    out.extend(commands.iter().map(|c| normalize_command(run, c)));
    out.sort_by(|a, b| a.ts_start.cmp(&b.ts_start));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewCommand, NewEvent, NewProject, NewRun};
    use crate::Store;

    fn seed(store: &Store) -> (Run, Vec<Event>, Vec<CommandRecord>) {
        let project = store
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/tmp/does-not-matter".into(),
                config_path: "/tmp/does-not-matter/.trace/config.toml".into(),
            })
            .unwrap();
        let run = store
            .create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: Some("claude-code".into()),
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap();
        store
            .add_event(
                &run.id,
                &NewEvent {
                    event_type: "risky_command_blocked".into(),
                    message: "blocked rm -rf /".into(),
                    metadata_json: Some(r#"{"command":"rm -rf /"}"#.into()),
                },
            )
            .unwrap();
        store
            .add_command(
                &run.id,
                &NewCommand {
                    command: "npm test".into(),
                    decision: "allow".into(),
                    exit_code: Some(0),
                    stdout_path: None,
                    stderr_path: None,
                },
            )
            .unwrap();
        let events = store.list_events(&run.id).unwrap();
        let commands = store.list_commands(&run.id).unwrap();
        (run, events, commands)
    }

    #[test]
    fn normalizes_events_and_commands_into_one_timeline() {
        let store = Store::open_in_memory().unwrap();
        let (run, events, commands) = seed(&store);
        let normalized = normalize_run(&run, &events, &commands);
        assert_eq!(normalized.len(), 2);
        assert!(normalized.iter().all(|e| e.actor == "claude-code"));
        assert!(normalized.iter().all(|e| e.source == "trace"));

        let blocked = normalized
            .iter()
            .find(|e| e.kind == "risky_command_blocked")
            .unwrap();
        assert_eq!(blocked.status, "blocked");
        assert_eq!(blocked.risk, "high");
        assert_eq!(blocked.target.as_deref(), Some("rm -rf /"));

        let cmd = normalized
            .iter()
            .find(|e| e.kind == "command_executed")
            .unwrap();
        assert_eq!(cmd.status, "ok");
        assert_eq!(cmd.risk, "none");
        assert_eq!(cmd.target.as_deref(), Some("npm test"));
    }

    #[test]
    fn unrecognized_event_kind_is_unknown_not_guessed() {
        let (status, risk) = status_and_risk_for_event_kind("some_future_event_kind");
        assert_eq!(status, EventStatus::Unknown);
        assert_eq!(risk, RiskLevel::None);
    }

    #[test]
    fn missing_agent_name_yields_literal_unknown_actor() {
        let store = Store::open_in_memory().unwrap();
        let project = store
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/tmp/x".into(),
                config_path: "/tmp/x/.trace/config.toml".into(),
            })
            .unwrap();
        let run = store
            .create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: None,
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap();
        assert_eq!(actor_for(&run), "unknown");
    }

    #[test]
    fn malformed_metadata_json_never_fabricates_an_object() {
        let metadata = parse_metadata(&Some("not json".to_string()));
        assert!(metadata.is_none());
        assert!(extract_target(&metadata).is_none());
    }
}
