//! `Incident` — a deterministic escalation of one or more high-severity
//! `Signal`s into the wire shape `apps/web/src/data.ts`'s `Incident`
//! interface expects from `GET /api/runs/:id/incidents`.
//!
//! Escalation policy (deliberately simple and stated here, not hidden in the
//! pipeline): every `Signal` at [`Severity::High`] becomes its own open
//! incident. Trace has no acknowledgement/resolution workflow yet, so
//! `status` is always the honest `"open"` — never a fabricated `"resolved"`.

use serde::{Deserialize, Serialize};

use crate::ids::short_hash;
use crate::intel::signal::Signal;
use crate::intel::NormalizedEvent;
use crate::policy::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: String,
    pub run_id: String,
    pub severity: Severity,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub signal_ids: Vec<String>,
    pub first_seen: String,
    pub last_seen: String,
}

/// Derive incidents from a run's signals + normalized timeline. Read-only,
/// pure function — no I/O, no mutation of any Trace state.
pub fn derive_incidents(
    run_id: &str,
    signals: &[Signal],
    events: &[NormalizedEvent],
) -> Vec<Incident> {
    signals
        .iter()
        .filter(|s| s.severity == Severity::High)
        .map(|s| {
            let (first_seen, last_seen) = evidence_time_span(s, events);
            Incident {
                id: format!("inc_{}", short_hash(&format!("{run_id}|{}", s.id))),
                run_id: run_id.to_string(),
                severity: s.severity,
                status: "open".to_string(),
                title: s.explanation.what.clone(),
                summary: s.explanation.why.clone(),
                signal_ids: vec![s.id.clone()],
                first_seen,
                last_seen,
            }
        })
        .collect()
}

/// The earliest/latest `ts_start` among a signal's evidence events, falling
/// back to the signal's own id-derived ordering only when no evidence event
/// matched (never guessed — an empty match yields `"unknown"`).
fn evidence_time_span(signal: &Signal, events: &[NormalizedEvent]) -> (String, String) {
    let mut times: Vec<&str> = events
        .iter()
        .filter(|e| signal.evidence_event_ids.contains(&e.id))
        .map(|e| e.ts_start.as_str())
        .collect();
    times.sort_unstable();
    match (times.first(), times.last()) {
        (Some(first), Some(last)) => (first.to_string(), last.to_string()),
        _ => ("unknown".to_string(), "unknown".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intel::signal::SignalExplanation;

    fn signal(id: &str, severity: Severity, evidence: Vec<&str>) -> Signal {
        Signal {
            id: id.to_string(),
            run_id: "r1".into(),
            kind: "retry_loop".into(),
            severity,
            confidence: 0.9,
            algorithm_id: "retry_loop_v1".into(),
            algorithm_version: "1.0.0".into(),
            evidence_event_ids: evidence.into_iter().map(|s| s.to_string()).collect(),
            explanation: SignalExplanation {
                what: "Repeated command".into(),
                why: "Same command ran 4 times in a row".into(),
                evidence: "evidence".into(),
                impact: "impact".into(),
                action: "action".into(),
            },
            observed: None,
            baseline: None,
            deviation: None,
            data_window: None,
        }
    }

    fn event(id: &str, ts: &str) -> NormalizedEvent {
        NormalizedEvent {
            id: id.to_string(),
            run_id: "r1".into(),
            parent_id: None,
            ts_start: ts.to_string(),
            ts_end: None,
            kind: "command_executed".into(),
            actor: "unknown".into(),
            source: "trace".into(),
            status: "ok".into(),
            risk: "none".into(),
            target: None,
            evidence: serde_json::json!({}),
            metadata: None,
        }
    }

    #[test]
    fn high_severity_signal_becomes_open_incident() {
        let signals = vec![signal("s1", Severity::High, vec!["e1", "e2"])];
        let events = vec![
            event("e1", "2026-01-01T00:00:00Z"),
            event("e2", "2026-01-01T00:05:00Z"),
        ];
        let incidents = derive_incidents("r1", &signals, &events);
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].status, "open");
        assert_eq!(incidents[0].signal_ids, vec!["s1".to_string()]);
        assert_eq!(incidents[0].first_seen, "2026-01-01T00:00:00Z");
        assert_eq!(incidents[0].last_seen, "2026-01-01T00:05:00Z");
    }

    #[test]
    fn medium_and_low_severity_signals_never_escalate() {
        let signals = vec![
            signal("s1", Severity::Medium, vec![]),
            signal("s2", Severity::Low, vec![]),
        ];
        let incidents = derive_incidents("r1", &signals, &[]);
        assert!(incidents.is_empty());
    }

    #[test]
    fn no_matching_evidence_events_yields_unknown_span_not_a_guess() {
        let signals = vec![signal("s1", Severity::High, vec!["missing"])];
        let incidents = derive_incidents("r1", &signals, &[]);
        assert_eq!(incidents[0].first_seen, "unknown");
        assert_eq!(incidents[0].last_seen, "unknown");
    }

    #[test]
    fn incident_ids_are_deterministic() {
        let signals = vec![signal("s1", Severity::High, vec![])];
        let a = derive_incidents("r1", &signals, &[]);
        let b = derive_incidents("r1", &signals, &[]);
        assert_eq!(a[0].id, b[0].id);
    }
}
