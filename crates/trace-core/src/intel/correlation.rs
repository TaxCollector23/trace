//! Signal correlation: groups related [`Signal`]s from one run into a single
//! [`Incident`] instead of a pile of separate alerts.
//!
//! # Grouping rule
//! Two signals are merged into the same incident when EITHER:
//! 1. their `evidence_event_ids` sets overlap (they already cite at least one
//!    of the same underlying events), OR
//! 2. they share at least one *target* — resolved from their evidence
//!    events' [`NormalizedEvent::target`] (a file path, a command line, a git
//!    ref) — AND their evidence time windows fall within
//!    [`TIME_WINDOW_SECS`] of each other.
//!
//! Two signals with **neither** an evidence overlap **nor** a shared target
//! are never merged, no matter how close in time they occurred — see
//! `two_unrelated_signals_never_merge` below. Grouping is transitive (union of
//! pairwise relations): if A merges with B and B merges with C, all three
//! land in one incident even if A and C have no direct relation.
//!
//! # Escalation threshold
//! A correlated group becomes an `Incident` only when its maximum severity is
//! at least [`Severity::Medium`]. A group made up entirely of `Low`-severity
//! signals — merged or not — is not yet incident-worthy; it stays visible as
//! plain signals. This generalizes the previous "High severity signals only"
//! rule: correlation itself (multiple detectors agreeing, or one detector's
//! evidence overlapping a shared target) is additional corroboration that a
//! `Medium` signal is a real pattern worth surfacing, not noise.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::ids::short_hash;
use crate::intel::incident::Incident;
use crate::intel::signal::Signal;
use crate::intel::NormalizedEvent;
use crate::policy::Severity;

/// How close two correlated-by-target signals' evidence windows must be (in
/// seconds) to still count as "the same underlying incident" rather than two
/// unrelated occurrences of work that happened to touch the same file or
/// command. 15 minutes: generous enough to span a slow edit/test/retry cycle,
/// tight enough that two signals from opposite ends of a long run never merge
/// on target alone.
const TIME_WINDOW_SECS: i64 = 15 * 60;

/// Per-signal facts derived once, used only for pairwise grouping decisions.
struct SignalFacts<'a> {
    evidence: HashSet<&'a str>,
    targets: HashSet<&'a str>,
    window: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

fn facts_for<'a>(
    signal: &'a Signal,
    events_by_id: &HashMap<&str, &'a NormalizedEvent>,
) -> SignalFacts<'a> {
    let evidence: HashSet<&str> = signal
        .evidence_event_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut targets: HashSet<&str> = HashSet::new();
    let mut times: Vec<DateTime<Utc>> = Vec::new();
    for id in &signal.evidence_event_ids {
        if let Some(ev) = events_by_id.get(id.as_str()) {
            if let Some(t) = ev.target.as_deref() {
                if !t.is_empty() {
                    targets.insert(t);
                }
            }
            if let Ok(dt) = DateTime::parse_from_rfc3339(&ev.ts_start) {
                times.push(dt.with_timezone(&Utc));
            }
        }
    }
    let window = match (times.iter().min(), times.iter().max()) {
        (Some(min), Some(max)) => Some((*min, *max)),
        _ => None,
    };
    SignalFacts {
        evidence,
        targets,
        window,
    }
}

fn windows_within(a: (DateTime<Utc>, DateTime<Utc>), b: (DateTime<Utc>, DateTime<Utc>)) -> bool {
    let gap = if a.1 < b.0 {
        (b.0 - a.1).num_seconds()
    } else if b.1 < a.0 {
        (a.0 - b.1).num_seconds()
    } else {
        0 // the windows already overlap
    };
    gap <= TIME_WINDOW_SECS
}

/// Whether two signals should be merged into the same incident. See module
/// docs for the exact rule — this is the single place that rule is enforced.
fn related(a: &SignalFacts, b: &SignalFacts) -> bool {
    if !a.evidence.is_disjoint(&b.evidence) {
        return true;
    }
    if a.targets.is_disjoint(&b.targets) {
        return false;
    }
    match (a.window, b.window) {
        (Some(wa), Some(wb)) => windows_within(wa, wb),
        _ => false,
    }
}

/// Minimal union-find over signal indices, used to turn the pairwise
/// `related` relation into connected components.
struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// Group `signals` into `Incident`s using the rule documented above. Pure,
/// read-only, deterministic: the same input always produces the same output
/// (including incident ids).
pub fn correlate_signals(
    run_id: &str,
    signals: &[Signal],
    events: &[NormalizedEvent],
) -> Vec<Incident> {
    if signals.is_empty() {
        return Vec::new();
    }

    let events_by_id: HashMap<&str, &NormalizedEvent> =
        events.iter().map(|e| (e.id.as_str(), e)).collect();
    let facts: Vec<SignalFacts> = signals
        .iter()
        .map(|s| facts_for(s, &events_by_id))
        .collect();

    let mut dsu = Dsu::new(signals.len());
    for i in 0..signals.len() {
        for j in (i + 1)..signals.len() {
            if related(&facts[i], &facts[j]) {
                dsu.union(i, j);
            }
        }
    }

    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..signals.len() {
        let root = dsu.find(i);
        groups.entry(root).or_default().push(i);
    }

    let mut incidents: Vec<Incident> = groups
        .into_values()
        .filter_map(|idxs| build_incident(run_id, &idxs, signals, &events_by_id))
        .collect();

    // Deterministic output ordering regardless of HashMap iteration order.
    incidents.sort_by(|a, b| a.first_seen.cmp(&b.first_seen).then(a.id.cmp(&b.id)));
    incidents
}

fn build_incident(
    run_id: &str,
    idxs: &[usize],
    signals: &[Signal],
    events_by_id: &HashMap<&str, &NormalizedEvent>,
) -> Option<Incident> {
    let group: Vec<&Signal> = idxs.iter().map(|&i| &signals[i]).collect();
    let severity = group.iter().map(|s| s.severity).max()?;
    if severity < Severity::Medium {
        return None;
    }

    let mut signal_ids: Vec<String> = group.iter().map(|s| s.id.clone()).collect();
    signal_ids.sort();

    let mut evidence_set: HashSet<String> = HashSet::new();
    for s in &group {
        evidence_set.extend(s.evidence_event_ids.iter().cloned());
    }
    let mut evidence: Vec<String> = evidence_set.into_iter().collect();
    evidence.sort();

    let mut times: Vec<&str> = evidence
        .iter()
        .filter_map(|id| events_by_id.get(id.as_str()))
        .map(|e| e.ts_start.as_str())
        .collect();
    times.sort_unstable();
    let (first_seen, last_seen) = match (times.first(), times.last()) {
        (Some(first), Some(last)) => (first.to_string(), last.to_string()),
        _ => ("unknown".to_string(), "unknown".to_string()),
    };

    let title = title_for_group(&group, events_by_id);
    let summary = summary_for_group(&group);

    Some(Incident {
        id: format!(
            "inc_{}",
            short_hash(&format!("{run_id}|{}", signal_ids.join(",")))
        ),
        run_id: run_id.to_string(),
        severity,
        status: "open".to_string(),
        title,
        summary,
        signal_ids,
        evidence,
        first_seen,
        last_seen,
    })
}

/// Human-readable phrase for a signal kind. Falls back to a title-cased
/// rendering of the raw kind string for any future analyzer this table
/// hasn't been updated for yet — never a blank/placeholder title.
fn kind_phrase(kind: &str) -> String {
    match kind {
        "retry_loop" => "Repeated command failures".to_string(),
        "unusual_execution_volume" => "Unusual command volume".to_string(),
        other => other
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn title_for_group(group: &[&Signal], events_by_id: &HashMap<&str, &NormalizedEvent>) -> String {
    let mut kinds: Vec<&str> = group.iter().map(|s| s.kind.as_str()).collect();
    kinds.sort_unstable();
    kinds.dedup();

    let phrase = if kinds.len() == 1 {
        kind_phrase(kinds[0])
    } else {
        format!(
            "{} related signals ({})",
            group.len(),
            kinds
                .iter()
                .map(|k| kind_phrase(k))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    // A target shared by every signal in the group (via their evidence
    // events) makes for a much more concrete title than the kind alone.
    let mut target_counts: HashMap<&str, usize> = HashMap::new();
    for s in group {
        let mut seen: HashSet<&str> = HashSet::new();
        for id in &s.evidence_event_ids {
            if let Some(ev) = events_by_id.get(id.as_str()) {
                if let Some(t) = ev.target.as_deref() {
                    if !t.is_empty() {
                        seen.insert(t);
                    }
                }
            }
        }
        for t in seen {
            *target_counts.entry(t).or_insert(0) += 1;
        }
    }
    let shared_target = target_counts
        .into_iter()
        .find(|&(_, count)| count == group.len())
        .map(|(t, _)| t);

    match shared_target {
        Some(t) => format!("{phrase} on {t}"),
        None => phrase,
    }
}

fn summary_for_group(group: &[&Signal]) -> String {
    if group.len() == 1 {
        return group[0].explanation.why.clone();
    }
    let parts: Vec<String> = group.iter().map(|s| s.explanation.what.clone()).collect();
    format!(
        "{} correlated signals describe one underlying pattern: {}",
        group.len(),
        parts.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intel::signal::{confidence, SignalExplanation};

    fn signal(
        id: &str,
        kind: &str,
        severity: Severity,
        evidence: Vec<&str>,
        confidence_val: f64,
    ) -> Signal {
        Signal {
            id: id.to_string(),
            run_id: "r1".into(),
            kind: kind.to_string(),
            severity,
            confidence: confidence_val,
            algorithm_id: format!("{kind}_v1"),
            algorithm_version: "1.0.0".into(),
            evidence_event_ids: evidence.into_iter().map(|s| s.to_string()).collect(),
            explanation: SignalExplanation {
                what: format!("{kind} observed"),
                why: format!("{kind} why"),
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

    fn event(id: &str, ts: &str, target: Option<&str>, kind: &str) -> NormalizedEvent {
        NormalizedEvent {
            id: id.to_string(),
            run_id: "r1".into(),
            parent_id: None,
            ts_start: ts.to_string(),
            ts_end: None,
            kind: kind.to_string(),
            actor: "claude-code".into(),
            source: "trace".into(),
            status: "ok".into(),
            risk: "none".into(),
            target: target.map(|t| t.to_string()),
            evidence: serde_json::json!({}),
            metadata: None,
        }
    }

    // --- Golden (a): shared evidence merges into one incident --------------
    #[test]
    fn golden_two_signals_sharing_evidence_merge_into_one_incident_with_union_fields() {
        let events = vec![
            event("e1", "2026-01-01T00:00:00Z", None, "command_executed"),
            event("e2", "2026-01-01T00:05:00Z", None, "command_executed"),
            event("e3", "2026-01-01T00:10:00Z", None, "command_executed"),
        ];
        let signals = vec![
            signal(
                "s1",
                "retry_loop",
                Severity::Medium,
                vec!["e1", "e2"],
                confidence::DETERMINISTIC,
            ),
            signal(
                "s2",
                "unusual_execution_volume",
                Severity::High,
                vec!["e2", "e3"],
                confidence::HEURISTIC_MAX,
            ),
        ];

        let incidents = correlate_signals("r1", &signals, &events);
        assert_eq!(incidents.len(), 1, "expected exactly one merged incident");
        let inc = &incidents[0];
        assert_eq!(
            inc.severity,
            Severity::High,
            "max of the grouped severities"
        );
        assert_eq!(inc.status, "open");
        let mut expected_signal_ids = vec!["s1".to_string(), "s2".to_string()];
        expected_signal_ids.sort();
        assert_eq!(inc.signal_ids, expected_signal_ids);
        let mut expected_evidence = vec!["e1".to_string(), "e2".to_string(), "e3".to_string()];
        expected_evidence.sort();
        assert_eq!(inc.evidence, expected_evidence, "union of evidence ids");
        assert_eq!(inc.first_seen, "2026-01-01T00:00:00Z");
        assert_eq!(inc.last_seen, "2026-01-01T00:10:00Z");
        assert!(!inc.title.is_empty());
    }

    // --- Golden (b): the false-merge guard ----------------------------------
    #[test]
    fn two_unrelated_signals_never_merge() {
        // No shared evidence_event_ids and no shared target: must stay two
        // separate incidents even though both are Medium+ severity.
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("npm test"),
                "command_executed",
            ),
            event(
                "e2",
                "2026-01-01T00:01:00Z",
                Some("cargo build"),
                "command_executed",
            ),
        ];
        let signals = vec![
            signal(
                "s1",
                "retry_loop",
                Severity::Medium,
                vec!["e1"],
                confidence::DETERMINISTIC,
            ),
            signal(
                "s2",
                "unusual_execution_volume",
                Severity::Medium,
                vec!["e2"],
                confidence::HEURISTIC_MIN,
            ),
        ];

        let incidents = correlate_signals("r1", &signals, &events);
        assert_eq!(
            incidents.len(),
            2,
            "unrelated signals must never be merged into one incident, got: {incidents:?}"
        );
        let all_signal_ids: Vec<&String> =
            incidents.iter().flat_map(|i| i.signal_ids.iter()).collect();
        assert!(all_signal_ids.contains(&&"s1".to_string()));
        assert!(all_signal_ids.contains(&&"s2".to_string()));
    }

    #[test]
    fn shared_target_within_time_window_merges_even_without_evidence_overlap() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("src/auth/session.ts"),
                "file_modified",
            ),
            event(
                "e2",
                "2026-01-01T00:05:00Z",
                Some("src/auth/session.ts"),
                "file_modified",
            ),
        ];
        let signals = vec![
            signal(
                "s1",
                "retry_loop",
                Severity::High,
                vec!["e1"],
                confidence::DETERMINISTIC,
            ),
            signal(
                "s2",
                "unusual_execution_volume",
                Severity::Medium,
                vec!["e2"],
                confidence::HEURISTIC_MIN,
            ),
        ];

        let incidents = correlate_signals("r1", &signals, &events);
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].signal_ids.len(), 2);
    }

    #[test]
    fn shared_target_outside_time_window_does_not_merge() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("src/auth/session.ts"),
                "file_modified",
            ),
            // Two hours later — same file, but far outside TIME_WINDOW_SECS.
            event(
                "e2",
                "2026-01-01T02:00:00Z",
                Some("src/auth/session.ts"),
                "file_modified",
            ),
        ];
        let signals = vec![
            signal(
                "s1",
                "retry_loop",
                Severity::High,
                vec!["e1"],
                confidence::DETERMINISTIC,
            ),
            signal(
                "s2",
                "unusual_execution_volume",
                Severity::Medium,
                vec!["e2"],
                confidence::HEURISTIC_MIN,
            ),
        ];

        let incidents = correlate_signals("r1", &signals, &events);
        assert_eq!(incidents.len(), 2);
    }

    #[test]
    fn low_severity_only_group_does_not_escalate_to_an_incident() {
        let events = vec![event(
            "e1",
            "2026-01-01T00:00:00Z",
            None,
            "command_executed",
        )];
        let signals = vec![signal(
            "s1",
            "retry_loop",
            Severity::Low,
            vec!["e1"],
            confidence::DETERMINISTIC,
        )];
        let incidents = correlate_signals("r1", &signals, &events);
        assert!(incidents.is_empty());
    }

    #[test]
    fn empty_signals_yields_no_incidents() {
        assert!(correlate_signals("r1", &[], &[]).is_empty());
    }

    #[test]
    fn incident_ids_are_deterministic() {
        let events = vec![event(
            "e1",
            "2026-01-01T00:00:00Z",
            None,
            "command_executed",
        )];
        let signals = vec![signal(
            "s1",
            "retry_loop",
            Severity::Medium,
            vec!["e1"],
            confidence::DETERMINISTIC,
        )];
        let a = correlate_signals("r1", &signals, &events);
        let b = correlate_signals("r1", &signals, &events);
        assert_eq!(a[0].id, b[0].id);
    }
}
