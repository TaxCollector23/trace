//! The algorithm registry: the `Analyzer` trait every detector implements,
//! its read-only input context, and the outcome type that forces a clean
//! choice between "here are signals" and "I cannot honestly say" instead of
//! ever emitting a wrong result on insufficient data.

use crate::intel::signal::Signal;
use crate::intel::NormalizedEvent;
use crate::models::{CommandRecord, Run};
use crate::Store;

/// Everything an analyzer is allowed to read. Deliberately read-only: no
/// method here can execute a command, write a file, or mutate any Trace
/// state — analyzers only ever look at what already happened.
pub struct AnalyzerContext<'a> {
    pub run: &'a Run,
    /// The run's full normalized timeline (events + commands), chronological.
    pub events: &'a [NormalizedEvent],
    /// The run's raw command rows (kept alongside `events` because analyzers
    /// that need exact command text/decision/exit-code find it more direct
    /// than re-parsing it back out of `NormalizedEvent::evidence`).
    pub commands: &'a [CommandRecord],
    /// The store, for analyzers that need cross-run baselines (e.g. prior
    /// comparable runs' command counts). Only ever used through `&Store`'s
    /// read methods.
    pub store: &'a Store,
}

/// Why an analyzer could not honestly produce a result — the required input
/// was absent or too thin, not that nothing was found. Carrying this (rather
/// than silently returning zero signals) is what lets a caller tell "checked,
/// found nothing" apart from "could not check".
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalyzerReport {
    pub algorithm_id: &'static str,
    pub algorithm_version: &'static str,
    /// `true` if the analyzer ran to completion (even if it found nothing);
    /// `false` if it declined for lack of sufficient input.
    pub ran: bool,
    /// Present exactly when `ran` is `false`: the specific reason, e.g.
    /// `"unavailable: no commands recorded for this run"`.
    pub unavailable_reason: Option<String>,
    pub signal_count: usize,
}

/// An analyzer's result for one run.
pub enum AnalyzerOutcome {
    /// Ran to completion. May be an empty vec — that is a real "checked,
    /// found nothing", not a missing result.
    Signals(Vec<Signal>),
    /// Declined: the required input was not available. `reason` is prefixed
    /// with `"unavailable: "` by convention so it reads unambiguously
    /// wherever it is logged or displayed.
    Unavailable { reason: String },
}

/// A deterministic detector. Every implementation must declare its identity
/// and inputs up front (`algorithm_id`, `algorithm_version`, `required_inputs`)
/// so the registry/report machinery never has to special-case a detector.
pub trait Analyzer {
    fn algorithm_id(&self) -> &'static str;
    fn algorithm_version(&self) -> &'static str;
    /// Human-readable description of what this analyzer needs to run at all
    /// (not what makes it *find* something — see each analyzer's module docs
    /// for the detection threshold itself).
    fn required_inputs(&self) -> &'static [&'static str];
    fn analyze(&self, ctx: &AnalyzerContext) -> AnalyzerOutcome;
}

/// The full set of registered analyzers. Adding a new one is a one-line
/// addition here; nothing else in the pipeline needs to change.
pub fn registered_analyzers() -> Vec<Box<dyn Analyzer>> {
    vec![
        Box::new(crate::intel::analyzers::retry_loop::RetryLoopAnalyzer),
        Box::new(crate::intel::analyzers::volume_anomaly::VolumeAnomalyAnalyzer),
    ]
}

/// Run every registered analyzer against `ctx`, returning the combined
/// signals plus a per-analyzer report (including any `Unavailable` reasons).
pub fn run_all(ctx: &AnalyzerContext) -> (Vec<Signal>, Vec<AnalyzerReport>) {
    let mut all_signals = Vec::new();
    let mut reports = Vec::new();
    for analyzer in registered_analyzers() {
        match analyzer.analyze(ctx) {
            AnalyzerOutcome::Signals(mut signals) => {
                reports.push(AnalyzerReport {
                    algorithm_id: analyzer.algorithm_id(),
                    algorithm_version: analyzer.algorithm_version(),
                    ran: true,
                    unavailable_reason: None,
                    signal_count: signals.len(),
                });
                all_signals.append(&mut signals);
            }
            AnalyzerOutcome::Unavailable { reason } => {
                reports.push(AnalyzerReport {
                    algorithm_id: analyzer.algorithm_id(),
                    algorithm_version: analyzer.algorithm_version(),
                    ran: false,
                    unavailable_reason: Some(format!("unavailable: {reason}")),
                    signal_count: 0,
                });
            }
        }
    }
    (all_signals, reports)
}
