//! Deterministic causality engine: "likely cause -> effect" chains between
//! events in one run's normalized timeline.
//!
//! # The mandate this module exists to enforce
//! A causality scorer must **never** claim causality from mere temporal
//! proximity alone. Every [`CausalLink`] this module produces requires at
//! least one *structural* signal in addition to (or, for a parent/child
//! relationship, in place of — see below) temporal proximity:
//! - `parent_child` — the effect event's `parent_id` names the cause event.
//!   Exact, deterministic (a real FK, not an inference) — sufficient on its
//!   own, no time-window check needed, and always reported at
//!   [`confidence::DETERMINISTIC`] (`HIGH`).
//! - `shared_file` / `shared_command` / `test_dependency` — the two events'
//!   [`NormalizedEvent::target`] match exactly (same file path, same command
//!   line, or both are test-kind events referencing the same target). These
//!   are heuristic, not proof of causation on their own (the same file can be
//!   touched hours apart by unrelated work), so they additionally require
//!   [`temporal_proximity`] to hold before they produce a claim at all, and
//!   are reported within the heuristic confidence band (`MEDIUM`/`LOW`),
//!   scaled by how close in time the two events are.
//!
//! `temporal_proximity` by itself — two events merely close in time, with
//! none of the above — **never** produces a [`CausalLink`]. See
//! `temporal_proximity_alone_never_produces_a_causal_link` below; this is the
//! single most important test in this module.
//!
//! # Wire shape decision
//! `CausalLink`/`EventCausality` are exposed as their own
//! `GET /api/runs/:id/causality` endpoint (`EventCausality[]`, one entry per
//! event that has at least one likely cause or effect) rather than folded
//! into `NormalizedEvent`. Reasons: (1) `NormalizedEvent` is a stable
//! superset-tolerant contract `apps/web/src/data.ts` already declares field
//! -by-field, and this repo's policy is not to touch `apps/web`, so adding an
//! optional field there would go undeclared on the TS side; (2) most events
//! have no causal links at all (this is by design — the mandate above is
//! deliberately conservative), so embedding an always-present-but-usually-
//! empty field on every event is wasted shape; a separate sparse endpoint
//! matches how `incidents` (also sparse) is already exposed.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::intel::signal::confidence;
use crate::intel::NormalizedEvent;

/// How close two events must be (in seconds) for `temporal_proximity` to
/// hold. Tighter than `correlation`'s window on purpose — a causal claim is a
/// stronger statement than "these belong to the same incident", so it should
/// demand a tighter temporal fit on top of the required structural evidence.
const TIME_WINDOW_SECS: i64 = 5 * 60;

/// Safety valve: pairwise comparison is O(n^2). For a very large run, only
/// the most recent `MAX_EVENTS_FOR_PAIRWISE` events (already chronologically
/// sorted by `mapper::normalize_run`) are considered — recent activity is
/// what a causality view is for, and this keeps the computation bounded.
const MAX_EVENTS_FOR_PAIRWISE: usize = 3000;

/// A single "this event is a likely cause/effect of that event" claim.
/// `event_id` names the *other* event in the relationship (the cause, when
/// this link is in an `EventCausality::likely_causes` list; the effect, when
/// it is in `likely_effects`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalLink {
    pub event_id: String,
    /// 0.0-1.0, see [`crate::intel::signal::confidence`] for the band basis.
    pub confidence: f64,
    /// Which structural signals fired, e.g. `["parent_child"]` or
    /// `["shared_file", "temporal_proximity"]`. Never `["temporal_proximity"]`
    /// alone — see module docs.
    pub basis: Vec<String>,
}

/// The causal links for one event: what likely caused it, and what it likely
/// caused in turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCausality {
    pub event_id: String,
    pub likely_causes: Vec<CausalLink>,
    pub likely_effects: Vec<CausalLink>,
}

/// Structural basis label for a shared, non-empty `target` between two
/// events, or `None` if they don't share one. Every case classifies into
/// exactly one of the three "shared X" bases from the module mandate — there
/// is no generic fallback label:
/// - both events are test-kind (`kind` contains `"test"`) referencing the
///   same target -> `test_dependency` (checked first: a test-to-test link is
///   more specific than "some file was involved");
/// - either event is file/directory-kind -> `shared_file` (covers the common
///   "this file edit is a likely cause of that later test failure
///   referencing the same path" case, even though the test event itself
///   isn't file-kind);
/// - otherwise (e.g. two command-executed events with the same command line)
///   -> `shared_command`.
fn shared_target_basis(a: &NormalizedEvent, b: &NormalizedEvent) -> Option<&'static str> {
    let (ta, tb) = (a.target.as_deref()?, b.target.as_deref()?);
    if ta.is_empty() || ta != tb {
        return None;
    }
    if a.kind.contains("test") && b.kind.contains("test") {
        return Some("test_dependency");
    }
    let is_file = |kind: &str| kind.contains("file") || kind.contains("directory");
    if is_file(&a.kind) || is_file(&b.kind) {
        return Some("shared_file");
    }
    Some("shared_command")
}

/// Exact structural parent/child linkage: `effect.parent_id == Some(cause.id)`.
/// Note `mapper::normalize_run` always sets `parent_id: None` against today's
/// schema (documented there) — this basis cannot fire on live data yet, but
/// the engine implements it now so a future schema change (or a future
/// `NormalizedEvent` source) lights it up with no changes here. Tested below
/// against directly-constructed events.
fn parent_child_basis(cause: &NormalizedEvent, effect: &NormalizedEvent) -> bool {
    effect.parent_id.as_deref() == Some(cause.id.as_str())
}

fn parse_ts(e: &NormalizedEvent) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&e.ts_start)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn time_gap_secs(a: &NormalizedEvent, b: &NormalizedEvent) -> Option<i64> {
    let (ta, tb) = (parse_ts(a)?, parse_ts(b)?);
    Some((tb - ta).num_seconds().abs())
}

/// Whether `a` and `b` occurred within [`TIME_WINDOW_SECS`] of each other.
/// An unparseable timestamp is treated as "not proximate" — the conservative
/// default, since this function gates whether a heuristic claim is allowed at
/// all.
fn temporal_proximity(a: &NormalizedEvent, b: &NormalizedEvent) -> bool {
    matches!(time_gap_secs(a, b), Some(gap) if gap <= TIME_WINDOW_SECS)
}

/// Evaluate whether `cause` (chronologically at or before `effect`) has a
/// causal relationship with `effect`, per the mandate in the module docs.
/// Returns `None` when the only evidence would be temporal proximity alone.
fn evaluate_link(cause: &NormalizedEvent, effect: &NormalizedEvent) -> Option<(f64, Vec<String>)> {
    if parent_child_basis(cause, effect) {
        let mut basis = vec!["parent_child".to_string()];
        if temporal_proximity(cause, effect) {
            basis.push("temporal_proximity".to_string());
        }
        return Some((confidence::DETERMINISTIC, basis));
    }

    let label = shared_target_basis(cause, effect)?;
    // Heuristic bases require temporal proximity IN ADDITION to the
    // structural match — never granted from the shared target alone, and
    // never granted from temporal proximity alone.
    if !temporal_proximity(cause, effect) {
        return None;
    }
    let gap = time_gap_secs(cause, effect).unwrap_or(TIME_WINDOW_SECS) as f64;
    let closeness = ((TIME_WINDOW_SECS as f64 - gap) / TIME_WINDOW_SECS as f64).clamp(0.0, 1.0);
    let conf = confidence::heuristic_scaled(closeness, 2.0);
    Some((
        conf,
        vec![label.to_string(), "temporal_proximity".to_string()],
    ))
}

/// Compute causal links across an already chronologically-sorted event
/// timeline (the contract `mapper::normalize_run` provides). Pure, read-only,
/// deterministic. Returns one [`EventCausality`] per event that has at least
/// one likely cause or effect — an event with neither is simply absent from
/// the result, never a placeholder empty entry.
pub fn compute_causal_links(events: &[NormalizedEvent]) -> Vec<EventCausality> {
    let scope: &[NormalizedEvent] = if events.len() > MAX_EVENTS_FOR_PAIRWISE {
        &events[events.len() - MAX_EVENTS_FOR_PAIRWISE..]
    } else {
        events
    };

    let mut causes_by_id: HashMap<&str, Vec<CausalLink>> = HashMap::new();
    let mut effects_by_id: HashMap<&str, Vec<CausalLink>> = HashMap::new();

    for i in 0..scope.len() {
        for j in (i + 1)..scope.len() {
            let cause = &scope[i];
            let effect = &scope[j];
            if cause.id == effect.id {
                continue;
            }
            if let Some((conf, basis)) = evaluate_link(cause, effect) {
                causes_by_id
                    .entry(effect.id.as_str())
                    .or_default()
                    .push(CausalLink {
                        event_id: cause.id.clone(),
                        confidence: conf,
                        basis: basis.clone(),
                    });
                effects_by_id
                    .entry(cause.id.as_str())
                    .or_default()
                    .push(CausalLink {
                        event_id: effect.id.clone(),
                        confidence: conf,
                        basis,
                    });
            }
        }
    }

    let mut ids: Vec<&str> = causes_by_id
        .keys()
        .chain(effects_by_id.keys())
        .copied()
        .collect();
    ids.sort_unstable();
    ids.dedup();

    ids.into_iter()
        .map(|id| {
            let mut likely_causes = causes_by_id.remove(id).unwrap_or_default();
            let mut likely_effects = effects_by_id.remove(id).unwrap_or_default();
            // Deterministic, most-confident-first ordering.
            let by_confidence_desc = |a: &CausalLink, b: &CausalLink| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.event_id.cmp(&b.event_id))
            };
            likely_causes.sort_by(by_confidence_desc);
            likely_effects.sort_by(by_confidence_desc);
            EventCausality {
                event_id: id.to_string(),
                likely_causes,
                likely_effects,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn event_with_parent(id: &str, parent: Option<&str>, ts: &str, kind: &str) -> NormalizedEvent {
        let mut e = event(id, ts, None, kind);
        e.parent_id = parent.map(|p| p.to_string());
        e
    }

    // --- Golden (d): the single most important test ------------------------
    #[test]
    fn temporal_proximity_alone_never_produces_a_causal_link() {
        // Two events one minute apart, no shared target, no parent/child.
        let events = vec![
            event("e1", "2026-01-01T00:00:00Z", None, "file_modified"),
            event("e2", "2026-01-01T00:01:00Z", None, "tests_started"),
        ];
        let links = compute_causal_links(&events);
        assert!(
            links.is_empty(),
            "temporal proximity alone must never yield a causal claim, got: {links:?}"
        );
    }

    #[test]
    fn temporal_proximity_alone_never_produces_a_link_even_with_different_targets() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("src/a.rs"),
                "file_modified",
            ),
            event(
                "e2",
                "2026-01-01T00:00:05Z",
                Some("src/b.rs"),
                "file_modified",
            ),
        ];
        let links = compute_causal_links(&events);
        assert!(links.is_empty());
    }

    // --- Golden (c): parent/child produces a HIGH-confidence claim ---------
    #[test]
    fn parent_child_relationship_produces_a_causal_link_with_parent_child_basis() {
        let events = vec![
            event_with_parent("e1", None, "2026-01-01T00:00:00Z", "command_started"),
            // Far apart in time on purpose: parent/child is exact structural
            // evidence and must not need temporal proximity to fire.
            event_with_parent("e2", Some("e1"), "2026-01-01T05:00:00Z", "command_output"),
        ];
        let links = compute_causal_links(&events);
        let e2 = links.iter().find(|l| l.event_id == "e2").unwrap();
        assert_eq!(e2.likely_causes.len(), 1);
        assert_eq!(e2.likely_causes[0].event_id, "e1");
        assert_eq!(e2.likely_causes[0].confidence, confidence::DETERMINISTIC);
        assert!(e2.likely_causes[0]
            .basis
            .contains(&"parent_child".to_string()));

        let e1 = links.iter().find(|l| l.event_id == "e1").unwrap();
        assert_eq!(e1.likely_effects.len(), 1);
        assert_eq!(e1.likely_effects[0].event_id, "e2");
    }

    // --- Golden (c): shared target + temporal proximity ---------------------
    #[test]
    fn shared_target_with_temporal_proximity_produces_a_medium_or_low_link() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("src/auth/session.ts"),
                "file_modified",
            ),
            event(
                "e2",
                "2026-01-01T00:02:00Z",
                Some("src/auth/session.ts"),
                "tests_failed",
            ),
        ];
        let links = compute_causal_links(&events);
        let e2 = links.iter().find(|l| l.event_id == "e2").unwrap();
        assert_eq!(e2.likely_causes.len(), 1);
        let link = &e2.likely_causes[0];
        assert_eq!(link.event_id, "e1");
        assert!(link.basis.contains(&"shared_file".to_string()));
        assert!(link.basis.contains(&"temporal_proximity".to_string()));
        assert!(link.confidence >= confidence::HEURISTIC_MIN);
        assert!(link.confidence <= confidence::HEURISTIC_MAX);
    }

    #[test]
    fn shared_target_outside_temporal_proximity_produces_no_link() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("src/auth/session.ts"),
                "file_modified",
            ),
            // An hour later: same file, but well outside TIME_WINDOW_SECS.
            event(
                "e2",
                "2026-01-01T01:00:00Z",
                Some("src/auth/session.ts"),
                "tests_failed",
            ),
        ];
        let links = compute_causal_links(&events);
        assert!(
            links.is_empty(),
            "shared target without temporal proximity must not produce a claim"
        );
    }

    #[test]
    fn shared_command_between_two_command_events_is_labeled_shared_command() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("npm run build"),
                "command_executed",
            ),
            event(
                "e2",
                "2026-01-01T00:00:30Z",
                Some("npm run build"),
                "command_executed",
            ),
        ];
        let links = compute_causal_links(&events);
        let e2 = links.iter().find(|l| l.event_id == "e2").unwrap();
        assert!(e2.likely_causes[0]
            .basis
            .contains(&"shared_command".to_string()));
    }

    #[test]
    fn shared_test_target_between_two_test_events_is_labeled_test_dependency() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("tests/integration/checkout.spec.ts"),
                "tests_started",
            ),
            event(
                "e2",
                "2026-01-01T00:00:20Z",
                Some("tests/integration/checkout.spec.ts"),
                "tests_failed",
            ),
        ];
        let links = compute_causal_links(&events);
        let e2 = links.iter().find(|l| l.event_id == "e2").unwrap();
        assert!(e2.likely_causes[0]
            .basis
            .contains(&"test_dependency".to_string()));
    }

    #[test]
    fn an_event_with_no_relationships_is_absent_from_the_result() {
        let events = vec![event("e1", "2026-01-01T00:00:00Z", None, "session_started")];
        assert!(compute_causal_links(&events).is_empty());
    }

    #[test]
    fn results_are_deterministic() {
        let events = vec![
            event(
                "e1",
                "2026-01-01T00:00:00Z",
                Some("src/x.rs"),
                "file_modified",
            ),
            event(
                "e2",
                "2026-01-01T00:01:00Z",
                Some("src/x.rs"),
                "file_modified",
            ),
        ];
        let a = compute_causal_links(&events);
        let b = compute_causal_links(&events);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }
}
