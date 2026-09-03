//! Retry-loop / oscillation detector.
//!
//! Detects a *stable, exact* repeated command (or short cycle of commands) at
//! the tail of a run's command history — the classic "agent keeps re-running
//! the same failing command" pattern, generalized to short oscillation cycles
//! (e.g. edit → test → edit → test with the same two commands each time).
//!
//! # Method
//! 1. Each command is reduced to a *stable signature*: whitespace-normalized
//!    text (the already-storage-redacted command string, collapsed to single
//!    spaces). No fuzzy matching — two *similar but not identical* commands
//!    never share a signature.
//! 2. For each candidate cycle length ("period") from 1 to
//!    [`MAX_PERIOD`], walk backward from the end of the run comparing
//!    adjacent `period`-sized blocks of signatures. Each matching pair
//!    extends the repeat count by one cycle.
//! 3. A period only becomes a candidate if its trailing repeat count is at
//!    least [`MIN_REPEATS`] full cycles — the minimum-evidence threshold.
//!    Among qualifying periods, the one with the highest repeat count wins
//!    (ties broken toward the smaller/simpler period).
//!
//! # False-positive guard
//! [`MIN_REPEATS`] = 3 means a period can only ever qualify when there are at
//! least `3 * period` trailing commands to compare — two commands (identical
//! or not) can never reach period 1's minimum of 3, so "two similar commands"
//! can never trigger this detector, by construction (prompt requirement).

use crate::ids::short_hash;
use crate::intel::registry::{Analyzer, AnalyzerContext, AnalyzerOutcome};
use crate::intel::signal::{confidence, SignalExplanation, SignalKind};
use crate::intel::Signal;
use crate::policy::Severity;

/// Maximum cycle length considered (a 5+-step "loop" is rare enough, and long
/// enough to usually be legitimate varied work, that we do not chase it).
const MAX_PERIOD: usize = 4;
/// Minimum number of full cycle repeats required before this is called a
/// loop at all — the minimum-evidence threshold. See module docs for why this
/// specific value is what keeps "two similar commands" from ever qualifying.
const MIN_REPEATS: usize = 3;

pub struct RetryLoopAnalyzer;

impl Analyzer for RetryLoopAnalyzer {
    fn algorithm_id(&self) -> &'static str {
        "retry_loop_v1"
    }

    fn algorithm_version(&self) -> &'static str {
        "1.0.0"
    }

    fn required_inputs(&self) -> &'static [&'static str] {
        &["run.commands"]
    }

    fn analyze(&self, ctx: &AnalyzerContext) -> AnalyzerOutcome {
        if ctx.commands.is_empty() {
            return AnalyzerOutcome::Unavailable {
                reason: "no commands recorded for this run".to_string(),
            };
        }

        let signatures: Vec<String> = ctx.commands.iter().map(|c| signature(&c.command)).collect();

        let Some((period, repeat_count)) = detect_trailing_repeats(&signatures) else {
            // Ran to completion, found nothing — a real negative, not a
            // missing result.
            return AnalyzerOutcome::Signals(vec![]);
        };

        let streak_len = period * repeat_count;
        let evidence_commands = &ctx.commands[ctx.commands.len() - streak_len..];
        let evidence_event_ids: Vec<String> =
            evidence_commands.iter().map(|c| c.id.clone()).collect();
        let cycle_text: Vec<String> = evidence_commands[..period]
            .iter()
            .map(|c| c.command.clone())
            .collect();

        let severity = if repeat_count >= 6 {
            Severity::High
        } else {
            Severity::Medium
        };

        let what = if period == 1 {
            format!(
                "The command `{}` ran {repeat_count} times in a row without the run finishing.",
                cycle_text[0]
            )
        } else {
            format!(
                "A {period}-command cycle repeated {repeat_count} times in a row: {}.",
                cycle_text.join(" -> ")
            )
        };

        let signal = Signal {
            id: format!(
                "sig_{}",
                short_hash(&format!(
                    "{}|retry_loop_v1|{}",
                    ctx.run.id,
                    evidence_event_ids.join(",")
                ))
            ),
            run_id: ctx.run.id.clone(),
            kind: SignalKind::RetryLoop.as_str().to_string(),
            severity,
            confidence: confidence::DETERMINISTIC,
            algorithm_id: self.algorithm_id().to_string(),
            algorithm_version: self.algorithm_version().to_string(),
            evidence_event_ids,
            explanation: SignalExplanation {
                what,
                why: format!(
                    "Trace's retry-loop detector found an exact repeated command sequence at \
                     the end of this run's command history: a {period}-command cycle repeated \
                     {repeat_count} times, at or above its minimum-evidence threshold of \
                     {MIN_REPEATS} repeats. Matching is exact text, not fuzzy — this never \
                     fires on merely similar commands."
                ),
                evidence: format!("Repeating cycle: {}", cycle_text.join(" -> ")),
                impact: "Repeating the same command (or short cycle) without a different outcome \
                    usually means the agent is stuck rather than making progress — time and \
                    tokens are being spent without new information."
                    .to_string(),
                action: "Review the repeated command's output and either fix the underlying \
                    failure or stop the run."
                    .to_string(),
            },
            observed: Some(serde_json::json!(repeat_count)),
            baseline: Some(serde_json::json!(MIN_REPEATS)),
            deviation: Some(serde_json::json!(repeat_count as i64 - MIN_REPEATS as i64)),
            data_window: Some(format!(
                "last {streak_len} commands ({repeat_count} repeats of a {period}-command cycle)"
            )),
        };

        AnalyzerOutcome::Signals(vec![signal])
    }
}

/// Whitespace-normalized, stable signature for a command string. Exact
/// equality only — no similarity scoring — so two commands that merely look
/// alike never collide.
fn signature(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Find the trailing periodic pattern with the strongest evidence. Returns
/// `(period, repeat_count)` for the winning period, or `None` if no period in
/// `1..=MAX_PERIOD` reaches [`MIN_REPEATS`] trailing repeats.
fn detect_trailing_repeats(signatures: &[String]) -> Option<(usize, usize)> {
    let n = signatures.len();
    let mut best: Option<(usize, usize)> = None;

    for period in 1..=MAX_PERIOD {
        if n < period * MIN_REPEATS {
            continue;
        }
        let mut repeat_count = 1usize;
        let mut pos = n;
        while pos >= 2 * period {
            let cur = &signatures[pos - period..pos];
            let prev = &signatures[pos - 2 * period..pos - period];
            if cur == prev {
                repeat_count += 1;
                pos -= period;
            } else {
                break;
            }
        }
        if repeat_count >= MIN_REPEATS {
            best = match best {
                Some((_, best_count)) if best_count >= repeat_count => best,
                _ => Some((period, repeat_count)),
            };
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommandRecord, NewProject, NewRun, Run};
    use crate::Store;

    fn make_run(store: &Store) -> Run {
        let project = store
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/tmp/retry-loop-test".into(),
                config_path: "/tmp/retry-loop-test/.trace/config.toml".into(),
            })
            .unwrap();
        store
            .create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: Some("claude-code".into()),
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap()
    }

    fn cmd(id: &str, run_id: &str, command: &str) -> CommandRecord {
        CommandRecord {
            id: id.into(),
            run_id: run_id.into(),
            command: command.into(),
            decision: "allow".into(),
            exit_code: Some(1),
            stdout_path: None,
            stderr_path: None,
            created_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn analyze(run: &Run, commands: &[CommandRecord], store: &Store) -> AnalyzerOutcome {
        let ctx = AnalyzerContext {
            run,
            events: &[],
            commands,
            store,
        };
        RetryLoopAnalyzer.analyze(&ctx)
    }

    // --- Golden: true retry loop -------------------------------------------
    #[test]
    fn golden_true_retry_loop_period_one() {
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        let commands: Vec<_> = (0..4)
            .map(|i| cmd(&format!("c{i}"), &run.id, "npm test"))
            .collect();
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => {
                assert_eq!(signals.len(), 1);
                let s = &signals[0];
                assert_eq!(s.kind, "retry_loop");
                assert_eq!(s.algorithm_id, "retry_loop_v1");
                assert_eq!(s.confidence, confidence::DETERMINISTIC);
                assert_eq!(s.evidence_event_ids.len(), 4);
                assert_eq!(s.observed, Some(serde_json::json!(4)));
            }
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    #[test]
    fn golden_true_retry_loop_oscillation_period_two() {
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        let texts = [
            "npm run build",
            "npm test",
            "npm run build",
            "npm test",
            "npm run build",
            "npm test",
        ];
        let commands: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| cmd(&format!("c{i}"), &run.id, t))
            .collect();
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => {
                assert_eq!(signals.len(), 1);
                assert_eq!(signals[0].observed, Some(serde_json::json!(3)));
            }
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    // --- Golden: not a loop --------------------------------------------------
    #[test]
    fn golden_not_a_loop_two_identical_commands_never_flags() {
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        let commands = vec![
            cmd("c0", &run.id, "npm test"),
            cmd("c1", &run.id, "npm test"),
        ];
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => assert!(signals.is_empty()),
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    #[test]
    fn golden_not_a_loop_similar_but_distinct_commands_never_flag() {
        // Same subsystem, different targets each time — never an exact
        // signature match, so this must never be flagged as a loop.
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        let commands: Vec<_> = ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"]
            .iter()
            .enumerate()
            .map(|(i, f)| cmd(&format!("c{i}"), &run.id, &format!("cargo test {f}")))
            .collect();
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => assert!(signals.is_empty()),
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    // --- Golden: normal large refactor (many distinct commands, no repeats) --
    #[test]
    fn golden_normal_large_refactor_many_distinct_commands_not_flagged() {
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        let commands: Vec<_> = (0..40)
            .map(|i| {
                cmd(
                    &format!("c{i}"),
                    &run.id,
                    &format!("git add src/file_{i}.rs"),
                )
            })
            .collect();
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => assert!(signals.is_empty()),
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    #[test]
    fn no_commands_is_explicitly_unavailable_not_a_false_negative() {
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        match analyze(&run, &[], &store) {
            AnalyzerOutcome::Unavailable { reason } => {
                assert!(reason.contains("no commands recorded"));
            }
            AnalyzerOutcome::Signals(_) => panic!("expected Unavailable for zero commands"),
        }
    }

    #[test]
    fn severity_escalates_to_high_at_six_or_more_repeats() {
        let store = Store::open_in_memory().unwrap();
        let run = make_run(&store);
        let commands: Vec<_> = (0..6)
            .map(|i| cmd(&format!("c{i}"), &run.id, "npm test"))
            .collect();
        match analyze(&run, &commands, &store) {
            AnalyzerOutcome::Signals(signals) => {
                assert_eq!(signals[0].severity, Severity::High);
            }
            AnalyzerOutcome::Unavailable { reason } => panic!("expected signals, got: {reason}"),
        }
    }

    #[test]
    fn detect_trailing_repeats_picks_strongest_period() {
        // 6 identical commands: period 1 gives repeat_count 6; period 2 (pairs
        // of the same command) would also match with repeat_count 3. Highest
        // repeat_count wins.
        let sigs: Vec<String> = vec!["x".into(); 6];
        assert_eq!(detect_trailing_repeats(&sigs), Some((1, 6)));
    }
}
