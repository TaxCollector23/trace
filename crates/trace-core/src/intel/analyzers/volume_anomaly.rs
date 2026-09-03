//! Anomalous command-volume detector, via robust statistics.
//!
//! Flags a run whose command count is a statistical outlier against a
//! per-project (optionally per-agent) baseline built from prior comparable
//! runs. Uses the median and MAD (median absolute deviation) rather than the
//! mean/standard-deviation specifically because a handful of past runaway
//! runs should not silently widen the baseline enough to hide the next one —
//! median/MAD are robust to exactly that kind of outlier contamination.
//!
//! # Method
//! 1. Pull command counts for prior runs in the same project (and, when the
//!    current run has an `agent_name`, the same agent) — read-only, via
//!    [`crate::db::Store::comparable_run_command_counts`].
//! 2. Require at least [`MIN_SAMPLE_SIZE`] prior runs. Fewer than that and the
//!    analyzer declines (`Unavailable`) rather than compute a baseline that
//!    is really just noise.
//! 3. Compute the modified z-score (Iglewicz & Hoaglin 1993):
//!    `z = 0.6745 * (observed - median) / MAD`. `MAD == 0` (every prior run
//!    executed identically many commands) is smoothed with
//!    [`MAD_EPSILON`] so this never divides by zero or reports an
//!    artificial infinite deviation for a one-command difference.
//! 4. `|z| >= `[`Z_THRESHOLD`]` is the standard robust-outlier cutoff; below
//!    it, this is a real (checked, not found) negative — not `Unavailable`.

use crate::ids::short_hash;
use crate::intel::registry::{Analyzer, AnalyzerContext, AnalyzerOutcome};
use crate::intel::signal::{confidence, SignalExplanation, SignalKind};
use crate::intel::Signal;
use crate::policy::Severity;

/// Minimum number of prior comparable runs required to trust a baseline at
/// all. Below this, "unusual" is not a meaningful claim.
const MIN_SAMPLE_SIZE: usize = 5;
/// Modified z-score magnitude beyond which an observation counts as a robust
/// statistical outlier (Iglewicz & Hoaglin's commonly cited cutoff).
const Z_THRESHOLD: f64 = 3.5;
/// Escalation cutoff: at or beyond twice the flagging threshold, this becomes
/// a `Severity::High` signal rather than `Medium`.
const HIGH_SEVERITY_Z: f64 = Z_THRESHOLD * 2.0;
/// Applied in place of a zero MAD (every prior run had an identical command
/// count) so the z-score computation never divides by zero and a
/// one-command difference cannot look like an infinite deviation.
const MAD_EPSILON: f64 = 1.0;
/// How many prior comparable runs to sample for the baseline.
const BASELINE_SAMPLE_LIMIT: i64 = 50;

pub struct VolumeAnomalyAnalyzer;

impl Analyzer for VolumeAnomalyAnalyzer {
    fn algorithm_id(&self) -> &'static str {
        "unusual_execution_volume_v1"
    }

    fn algorithm_version(&self) -> &'static str {
        "1.0.0"
    }

    fn required_inputs(&self) -> &'static [&'static str] {
        &["prior comparable runs' command counts (same project, optionally same agent)"]
    }

    fn analyze(&self, ctx: &AnalyzerContext) -> AnalyzerOutcome {
        let observed = ctx.commands.len() as f64;

        let prior = match ctx.store.comparable_run_command_counts(
            &ctx.run.project_id,
            ctx.run.agent_name.as_deref(),
            &ctx.run.id,
            BASELINE_SAMPLE_LIMIT,
        ) {
            Ok(v) => v,
            Err(e) => {
                return AnalyzerOutcome::Unavailable {
                    reason: format!("could not read prior comparable runs: {e}"),
                }
            }
        };

        if prior.len() < MIN_SAMPLE_SIZE {
            return AnalyzerOutcome::Unavailable {
                reason: format!(
                    "insufficient prior run history for a baseline ({} prior comparable run(s), \
                     need at least {MIN_SAMPLE_SIZE})",
                    prior.len()
                ),
            };
        }

        let values: Vec<f64> = prior.iter().map(|&c| c as f64).collect();
        let baseline_median = median(&values);
        let raw_mad = mad(&values, baseline_median);
        let effective_mad = if raw_mad == 0.0 { MAD_EPSILON } else { raw_mad };
        let z = 0.6745 * (observed - baseline_median) / effective_mad;

        if z.abs() < Z_THRESHOLD {
            // Ran, checked, found nothing unusual — a real negative.
            return AnalyzerOutcome::Signals(vec![]);
        }

        let severity = if z.abs() >= HIGH_SEVERITY_Z {
            Severity::High
        } else {
            Severity::Medium
        };
        let direction = if z > 0.0 { "more" } else { "fewer" };
        let evidence_event_ids: Vec<String> = ctx.commands.iter().map(|c| c.id.clone()).collect();
        let agent_suffix = ctx
            .run
            .agent_name
            .as_ref()
            .map(|a| format!(" and agent {a}"))
            .unwrap_or_default();

        let signal = Signal {
            id: format!(
                "sig_{}",
                short_hash(&format!(
                    "{}|unusual_execution_volume_v1|{}",
                    ctx.run.id, observed
                ))
            ),
            run_id: ctx.run.id.clone(),
            kind: SignalKind::UnusualExecutionVolume.as_str().to_string(),
            severity,
            confidence: confidence::heuristic_scaled(z.abs() - Z_THRESHOLD, 0.3),
            algorithm_id: self.algorithm_id().to_string(),
            algorithm_version: self.algorithm_version().to_string(),
            evidence_event_ids,
            explanation: SignalExplanation {
                what: format!(
                    "This run executed {} commands — far {direction} than typical for this \
                     project{agent_suffix}.",
                    observed as i64
                ),
                why: {
                    let sample_size = prior.len();
                    format!(
                        "Robust baseline over {sample_size} prior comparable run(s): median \
                         {baseline_median} commands, MAD {raw_mad:.2}. This run's modified \
                         z-score is {z:.2}, past the {Z_THRESHOLD} threshold used for a \
                         statistical outlier."
                    )
                },
                evidence: {
                    let observed_i = observed as i64;
                    let sample_size = prior.len();
                    format!(
                        "Observed {observed_i} commands vs. a baseline median of \
                         {baseline_median} (MAD {raw_mad:.2}) over {sample_size} prior run(s)."
                    )
                },
                impact: "A run executing far more (or fewer) commands than is typical for this \
                    project/agent can indicate a stuck loop, runaway automation, or scope well \
                    beyond the intended task."
                    .to_string(),
                action: "Review this run's command list for unexpected repetition or scope \
                    drift beyond what was asked."
                    .to_string(),
            },
            observed: Some(serde_json::json!(observed as i64)),
            baseline: Some(serde_json::json!({
                "median": baseline_median,
                "mad": raw_mad,
                "sample_size": prior.len(),
            })),
            deviation: Some(serde_json::json!(z)),
            data_window: Some(format!(
                "last {} comparable run(s) for this project{agent_suffix}",
                prior.len()
            )),
        };

        AnalyzerOutcome::Signals(vec![signal])
    }
}

fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).expect("finite command counts"));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn mad(values: &[f64], center: f64) -> f64 {
    let deviations: Vec<f64> = values.iter().map(|v| (v - center).abs()).collect();
    median(&deviations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommandRecord, NewCommand, NewProject, NewRun, Run};
    use crate::Store;

    fn seed_project_and_target_run(
        store: &Store,
        agent: Option<&str>,
        target_command_count: usize,
    ) -> Run {
        let project = store
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/tmp/volume-anomaly-test".into(),
                config_path: "/tmp/volume-anomaly-test/.trace/config.toml".into(),
            })
            .unwrap();
        let run = store
            .create_run(&NewRun {
                project_id: project.id.clone(),
                command: "run".into(),
                agent_name: agent.map(|a| a.to_string()),
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap();
        for i in 0..target_command_count {
            store
                .add_command(
                    &run.id,
                    &NewCommand {
                        command: format!("cmd {i}"),
                        decision: "allow".into(),
                        exit_code: Some(0),
                        stdout_path: None,
                        stderr_path: None,
                    },
                )
                .unwrap();
        }
        run
    }

    fn seed_prior_run(store: &Store, project_id: &str, agent: Option<&str>, n_commands: usize) {
        let run = store
            .create_run(&NewRun {
                project_id: project_id.to_string(),
                command: "run".into(),
                agent_name: agent.map(|a| a.to_string()),
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap();
        for i in 0..n_commands {
            store
                .add_command(
                    &run.id,
                    &NewCommand {
                        command: format!("cmd {i}"),
                        decision: "allow".into(),
                        exit_code: Some(0),
                        stdout_path: None,
                        stderr_path: None,
                    },
                )
                .unwrap();
        }
    }

    fn commands_for(store: &Store, run_id: &str) -> Vec<CommandRecord> {
        store.list_commands(run_id).unwrap()
    }

    fn analyze(run: &Run, commands: &[CommandRecord], store: &Store) -> AnalyzerOutcome {
        let ctx = AnalyzerContext {
            run,
            events: &[],
            commands,
            store,
        };
        VolumeAnomalyAnalyzer.analyze(&ctx)
    }

    #[test]
    fn insufficient_baseline_is_explicitly_unavailable() {
        let store = Store::open_in_memory().unwrap();
        let run = seed_project_and_target_run(&store, Some("claude"), 40);
        // Only 2 prior comparable runs — below MIN_SAMPLE_SIZE.
        seed_prior_run(&store, &run.project_id, Some("claude"), 5);
        seed_prior_run(&store, &run.project_id, Some("claude"), 6);

        let commands = commands_for(&store, &run.id);
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Unavailable { reason } => {
                assert!(reason.contains("insufficient prior run history"));
            }
            AnalyzerOutcome::Signals(_) => panic!("expected Unavailable"),
        }
    }

    // --- Golden: true anomaly -------------------------------------------
    #[test]
    fn golden_true_anomaly_flagged_against_tight_baseline() {
        let store = Store::open_in_memory().unwrap();
        let run = seed_project_and_target_run(&store, Some("claude"), 40);
        for n in [5, 6, 5, 7, 6, 5, 6] {
            seed_prior_run(&store, &run.project_id, Some("claude"), n);
        }

        let commands = commands_for(&store, &run.id);
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => {
                assert_eq!(signals.len(), 1);
                let s = &signals[0];
                assert_eq!(s.kind, "unusual_execution_volume");
                assert_eq!(s.algorithm_id, "unusual_execution_volume_v1");
                assert!(s.confidence >= confidence::HEURISTIC_MIN);
                assert!(s.confidence <= confidence::HEURISTIC_MAX);
                assert_eq!(s.observed, Some(serde_json::json!(40)));
                assert_eq!(s.evidence_event_ids.len(), 40);
            }
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    // --- Golden: normal large refactor (baseline itself has wide spread) --
    #[test]
    fn golden_normal_large_refactor_not_flagged_despite_large_absolute_count() {
        let store = Store::open_in_memory().unwrap();
        let run = seed_project_and_target_run(&store, Some("claude"), 50);
        // A heterogeneous history: some small runs, some big ones. The spread
        // (MAD) is wide, so 50 commands is not actually unusual here.
        for n in [10, 60, 15, 55, 12, 58, 20, 50] {
            seed_prior_run(&store, &run.project_id, Some("claude"), n);
        }

        let commands = commands_for(&store, &run.id);
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => assert!(signals.is_empty()),
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    #[test]
    fn zero_mad_is_smoothed_not_a_divide_by_zero() {
        let store = Store::open_in_memory().unwrap();
        let run = seed_project_and_target_run(&store, Some("claude"), 12);
        for _ in 0..5 {
            seed_prior_run(&store, &run.project_id, Some("claude"), 5);
        }
        let commands = commands_for(&store, &run.id);
        // z = 0.6745 * (12-5) / 1.0 (MAD_EPSILON) = 4.72 -> flagged.
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => assert_eq!(signals.len(), 1),
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    #[test]
    fn agent_scoping_excludes_other_agents_from_the_baseline() {
        let store = Store::open_in_memory().unwrap();
        let run = seed_project_and_target_run(&store, Some("claude"), 6);
        for n in [5, 6, 5, 7, 6] {
            seed_prior_run(&store, &run.project_id, Some("claude"), n);
        }
        // A different agent running wildly larger volumes must not pollute
        // claude's baseline.
        for _ in 0..10 {
            seed_prior_run(&store, &run.project_id, Some("codex"), 500);
        }
        let commands = commands_for(&store, &run.id);
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => assert!(signals.is_empty()),
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }
}
