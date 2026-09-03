//! `Signal` — the wire shape `apps/web/src/data.ts`'s `Signal` interface
//! expects from `GET /api/runs/:id/signals`, plus the algorithm-id taxonomy
//! and the confidence-basis policy every analyzer must follow.

use serde::{Deserialize, Serialize};

use crate::policy::Severity;

/// Confidence bands with a DEFINED basis (prompt requirement): a signal built
/// from an exact, deterministic pattern/threshold match reports `DETERMINISTIC`
/// (high); a signal built from a statistical heuristic (robust-stats outlier,
/// etc.) reports `HEURISTIC` (medium) with confidence scaled *within* that
/// band by the strength of the deviation — never above it, since a heuristic
/// is never as certain as an exact match.
pub mod confidence {
    /// Deterministic policy/pattern match (e.g. an exact repeated-command
    /// subsequence at or above the minimum-evidence threshold). Fixed, not
    /// scaled — the match either holds or it doesn't.
    pub const DETERMINISTIC: f64 = 0.92;

    /// Lower bound of the heuristic (statistical) band.
    pub const HEURISTIC_MIN: f64 = 0.5;
    /// Upper bound of the heuristic band. Deliberately below
    /// [`DETERMINISTIC`] — a heuristic never outranks an exact match.
    pub const HEURISTIC_MAX: f64 = 0.85;

    /// Scale a non-negative "how far past the threshold" magnitude into the
    /// heuristic band. `steepness` controls how quickly it saturates.
    pub fn heuristic_scaled(magnitude_past_threshold: f64, steepness: f64) -> f64 {
        let m = magnitude_past_threshold.max(0.0);
        let span = HEURISTIC_MAX - HEURISTIC_MIN;
        (HEURISTIC_MIN + span * (1.0 - (-m * steepness).exp())).clamp(HEURISTIC_MIN, HEURISTIC_MAX)
    }
}

/// Closed taxonomy of signal kinds this codebase currently knows how to
/// produce. `apps/web`'s `Signal.kind` type is `string` (forward-compatible),
/// so adding a new analyzer never requires a UI change — but every analyzer
/// that exists should register its kind here so the taxonomy stays a single
/// source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    /// A stable command (or short cycle of commands) repeating without
    /// progress — see `analyzers::retry_loop`.
    RetryLoop,
    /// A run's command volume falls far outside the robust baseline for
    /// comparable prior runs — see `analyzers::volume_anomaly`.
    UnusualExecutionVolume,
}

impl SignalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalKind::RetryLoop => "retry_loop",
            SignalKind::UnusualExecutionVolume => "unusual_execution_volume",
        }
    }
}

/// The "why am I seeing this?" breakdown the UI renders verbatim
/// (`v4/SignalCard.tsx`). Every field must be a real, evidence-backed
/// sentence — never a templated guess about *this specific instance* beyond
/// what the algorithm actually computed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalExplanation {
    pub what: String,
    pub why: String,
    pub evidence: String,
    pub impact: String,
    pub action: String,
}

/// A single detector output. See `apps/web/src/data.ts`'s `Signal` interface
/// — field names and optionality here are matched exactly so the UI lights up
/// without further changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: String,
    pub run_id: String,
    pub kind: String,
    pub severity: Severity,
    /// 0.0-1.0, see the `confidence` module for the band an analyzer must use.
    pub confidence: f64,
    pub algorithm_id: String,
    pub algorithm_version: String,
    /// IDs into `NormalizedEvent`/`CommandRecord` rows that back this signal.
    pub evidence_event_ids: Vec<String>,
    pub explanation: SignalExplanation,
    pub observed: Option<serde_json::Value>,
    pub baseline: Option<serde_json::Value>,
    pub deviation: Option<serde_json::Value>,
    pub data_window: Option<String>,
}
