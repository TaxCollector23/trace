//! Deterministic intelligence spine (Wave 1, Agent 2).
//!
//! Everything under `intel/` is deterministic: no LLM, no network call, no
//! guessing. Every analyzer declares the inputs it needs and a defined basis
//! for its confidence score; when the inputs it needs are not available it
//! says so explicitly (`AnalyzerOutcome::Unavailable`) instead of emitting a
//! signal built on data that was not actually there.
//!
//! Module map:
//! - [`event`] — the normalized event model (`NormalizedEvent`) all analyzers
//!   read from, plus the `EventStatus`/`RiskLevel` taxonomies.
//! - [`mapper`] — turns the *current* stored shape (`Event` rows + `CommandRecord`
//!   rows) into `NormalizedEvent`s. If the DB cannot supply a field the mapper
//!   leaves it `None`/`Unknown`; it never invents one.
//! - [`signal`] — `Signal`, `SignalExplanation`, `SignalKind`, and the two
//!   confidence bands (`confidence::DETERMINISTIC` / `confidence::HEURISTIC`).
//! - [`incident`] — the `Incident` wire shape.
//! - [`correlation`] (Wave 2) — groups related `Signal`s into `Incident`s by
//!   evidence overlap or shared target + tight time window; never merges on
//!   coincidence alone.
//! - [`causality`] (Wave 2) — deterministic "likely cause -> effect" chains
//!   between events, gated so temporal proximity alone never produces a
//!   claim.
//! - [`registry`] — the `Analyzer` trait, `AnalyzerContext`, `AnalyzerOutcome`.
//! - [`analyzers`] — the concrete analyzers (retry-loop, volume anomaly).
//! - [`pipeline`] — wires a `Store` + run id through mapping → analysis →
//!   correlation → causality. This is what the daemon route handlers call.

pub mod analyzers;
pub mod causality;
pub mod correlation;
pub mod event;
pub mod incident;
pub mod mapper;
pub mod pipeline;
pub mod registry;
pub mod signal;

pub use causality::{compute_causal_links, CausalLink, EventCausality};
pub use correlation::correlate_signals;
pub use event::{EventStatus, NormalizedEvent, RiskLevel};
pub use incident::Incident;
pub use mapper::normalize_run;
pub use pipeline::{run_intel_pipeline, IntelBundle};
pub use registry::{Analyzer, AnalyzerContext, AnalyzerOutcome, AnalyzerReport};
pub use signal::{confidence, Signal, SignalExplanation, SignalKind};
