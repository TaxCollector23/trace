//! Wires a `Store` + run id through mapping -> analysis -> incident
//! derivation. This is the single entry point the daemon's read-only intel
//! routes call; nothing else in `trace-daemon` needs to know how the pieces
//! fit together.

use anyhow::Result;

use crate::intel::incident::{self, Incident};
use crate::intel::mapper;
use crate::intel::registry::{self, AnalyzerContext, AnalyzerReport};
use crate::intel::{NormalizedEvent, Signal};
use crate::Store;

/// The full computed intelligence picture for one run.
pub struct IntelBundle {
    pub events: Vec<NormalizedEvent>,
    pub signals: Vec<Signal>,
    pub incidents: Vec<Incident>,
    /// Per-analyzer status, including any `unavailable: <reason>` — not part
    /// of the wire contract the UI reads, but available to callers/tests that
    /// want to distinguish "checked, found nothing" from "could not check".
    pub reports: Vec<AnalyzerReport>,
}

/// Compute the intel bundle for `run_id`. Returns `Ok(None)` when the run
/// itself does not exist (the caller's cue to answer 404), `Err` only for a
/// genuine storage failure. Entirely read-only: no command execution, no file
/// writes, no mutation of any Trace state.
pub fn run_intel_pipeline(store: &Store, run_id: &str) -> Result<Option<IntelBundle>> {
    let Some(run) = store.run_by_id(run_id)? else {
        return Ok(None);
    };
    let raw_events = store.list_events(run_id)?;
    let commands = store.list_commands(run_id)?;
    let events = mapper::normalize_run(&run, &raw_events, &commands);

    let ctx = AnalyzerContext {
        run: &run,
        events: &events,
        commands: &commands,
        store,
    };
    let (signals, reports) = registry::run_all(&ctx);
    let incidents = incident::derive_incidents(run_id, &signals, &events);

    Ok(Some(IntelBundle {
        events,
        signals,
        incidents,
        reports,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewCommand, NewProject, NewRun};

    #[test]
    fn unknown_run_returns_none_not_an_error() {
        let store = Store::open_in_memory().unwrap();
        let result = run_intel_pipeline(&store, "does-not-exist").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn pipeline_ties_events_signals_and_incidents_together() {
        let store = Store::open_in_memory().unwrap();
        let project = store
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/tmp/pipeline-test".into(),
                config_path: "/tmp/pipeline-test/.trace/config.toml".into(),
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
        // Six repeats of the same command -> retry_loop, repeat_count 6 -> High.
        for i in 0..6 {
            store
                .add_command(
                    &run.id,
                    &NewCommand {
                        command: "npm test".into(),
                        decision: "allow".into(),
                        exit_code: Some(1),
                        stdout_path: None,
                        stderr_path: None,
                    },
                )
                .unwrap();
            let _ = i;
        }

        let bundle = run_intel_pipeline(&store, &run.id).unwrap().unwrap();
        assert_eq!(bundle.events.len(), 6);
        assert_eq!(bundle.signals.len(), 1);
        assert_eq!(bundle.signals[0].kind, "retry_loop");
        // High severity (6 repeats) escalates to an incident.
        assert_eq!(bundle.incidents.len(), 1);
        assert_eq!(
            bundle.incidents[0].signal_ids,
            vec![bundle.signals[0].id.clone()]
        );
        // The volume analyzer had no prior comparable runs -> unavailable.
        let volume_report = bundle
            .reports
            .iter()
            .find(|r| r.algorithm_id == "unusual_execution_volume_v1")
            .unwrap();
        assert!(!volume_report.ran);
        assert!(volume_report
            .unavailable_reason
            .as_deref()
            .unwrap()
            .starts_with("unavailable:"));
    }
}
