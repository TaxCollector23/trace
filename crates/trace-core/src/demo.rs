//! Deterministic demo/scenario data generator for `trc demo`.
//!
//! The product owner's real question was "how can I test the dashboard again
//! without needing a real coding agent running?" This module answers that: it
//! builds a fully realistic, deterministic run (commands, events, file
//! changes, test results — a coherent narrative) straight through the same
//! [`Store`] API a real `trc run` would use, so `trc dashboard` immediately
//! shows something real-looking to click through.
//!
//! Every row this module writes is tagged as synthetic (see
//! [`tagged_metadata`]) so a future cleanup pass — or `trc reset
//! --local-data`, which already purges every run regardless of origin — can
//! tell it apart from a user's real telemetry. Nothing here touches git,
//! executes a real command, or calls the network: every "command" and "file
//! change" is a plain data row describing what *would* have happened, built
//! with the same [`NewCommand`]/[`NewEvent`]/[`NewFileChange`] shapes real
//! runs use.

use anyhow::Result;
use serde_json::{json, Value};

use crate::guard;
use crate::models::*;
use crate::Store;

/// One selectable demo scenario.
pub struct ScenarioInfo {
    pub name: &'static str,
    pub description: &'static str,
}

/// Every scenario `trc demo` knows how to generate, in the order `trc demo
/// list` prints them.
pub const SCENARIOS: &[ScenarioInfo] = &[
    ScenarioInfo {
        name: "normal_feature",
        description:
            "A clean multi-command run: read files, edit, tests pass. Terminal outcome SUCCESS.",
    },
    ScenarioInfo {
        name: "failed_test_repair",
        description:
            "Edit, test fails, edit again, test passes — a realistic repair loop (too short to trip the retry-loop analyzer, by design).",
    },
    ScenarioInfo {
        name: "dangerous_command",
        description:
            "The agent attempts a destructive command; the real command guard blocks it. Terminal outcome BLOCKED.",
    },
    ScenarioInfo {
        name: "retry_loop",
        description:
            "The same command repeated unchanged 4 times — deliberately trips the intel spine's retry_loop_v1 analyzer.",
    },
];

/// Look up a scenario by name.
pub fn find_scenario(name: &str) -> Option<&'static ScenarioInfo> {
    SCENARIOS.iter().find(|s| s.name == name)
}

/// Build the named scenario against `store`, under `project_id`, seeded by
/// `seed` (affects only timestamp jitter — the event sequence per scenario is
/// fixed). Returns the created [`Run`]. Errors if `name` is not a known
/// scenario.
pub fn run_scenario(store: &Store, project_id: &str, name: &str, seed: u64) -> Result<Run> {
    match name {
        "normal_feature" => normal_feature(store, project_id, seed),
        "failed_test_repair" => failed_test_repair(store, project_id, seed),
        "dangerous_command" => dangerous_command(store, project_id, seed),
        "retry_loop" => retry_loop(store, project_id, seed),
        other => {
            let names: Vec<&str> = SCENARIOS.iter().map(|s| s.name).collect();
            anyhow::bail!(
                "unknown demo scenario '{other}'. Available: {}. Run `trc demo list` for details.",
                names.join(", ")
            )
        }
    }
}

// --- Deterministic timestamp clock -----------------------------------------

/// A tiny seeded PRNG (SplitMix64) used only to jitter demo timestamps —
/// never to decide which events a scenario produces, so the sequence stays
/// fixed for a given scenario regardless of seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero fixed point.
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `[lo, hi]` inclusive.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }
}

/// Hands out strictly increasing, staggered RFC3339 timestamps for one
/// scenario's rows. Anchored a little in the past so the generated run reads
/// as "just happened" rather than in the future.
struct Clock {
    at: chrono::DateTime<chrono::Utc>,
    rng: Rng,
}

impl Clock {
    fn new(seed: u64) -> Self {
        Clock {
            at: chrono::Utc::now() - chrono::Duration::minutes(15),
            rng: Rng::new(seed),
        }
    }

    /// Advance by 1-6 seconds (seed-jittered) and return the new timestamp.
    fn tick(&mut self) -> String {
        let jitter = self.rng.range(1, 6) as i64;
        self.at += chrono::Duration::seconds(jitter);
        self.at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

// --- Synthetic tagging -------------------------------------------------------

/// Merge `{"synthetic": true, "scenario": "<name>"}` into `extra` (or start
/// fresh from `{}`), producing the `metadata_json` string every demo-generated
/// event carries. This is the marker a future cleanup/filter step (or the
/// dashboard) can use to distinguish `trc demo` rows from real telemetry —
/// e.g. `SELECT DISTINCT run_id FROM events WHERE metadata_json LIKE
/// '%"synthetic":true%'`.
fn tagged_metadata(scenario: &str, extra: Option<Value>) -> String {
    let mut obj = extra.unwrap_or_else(|| json!({}));
    if let Some(map) = obj.as_object_mut() {
        map.insert("synthetic".to_string(), json!(true));
        map.insert("scenario".to_string(), json!(scenario));
    }
    obj.to_string()
}

/// Append a tagged event.
fn ev(
    store: &Store,
    run_id: &str,
    scenario: &str,
    clock: &mut Clock,
    kind: EventType,
    message: impl Into<String>,
    extra_metadata: Option<Value>,
) -> Result<()> {
    let at = clock.tick();
    store.add_event_at(
        run_id,
        &NewEvent {
            event_type: kind.as_str().to_string(),
            message: message.into(),
            metadata_json: Some(tagged_metadata(scenario, extra_metadata)),
        },
        &at,
    )?;
    Ok(())
}

/// The first event of every demo run: a plain, human-readable marker so
/// anyone looking at the timeline immediately understands this run is
/// synthetic, not a fabricated real session.
fn note_synthetic(store: &Store, run_id: &str, scenario: &str, clock: &mut Clock) -> Result<()> {
    ev(
        store,
        run_id,
        scenario,
        clock,
        EventType::Note,
        format!(
            "Synthetic demo run (scenario: {scenario}), generated by `trc demo`. \
             Safe to remove with `trc reset --local-data`."
        ),
        None,
    )
}

/// Append a command row.
fn cmd(
    store: &Store,
    run_id: &str,
    clock: &mut Clock,
    command: impl Into<String>,
    decision: &str,
    exit_code: Option<i64>,
) -> Result<()> {
    let at = clock.tick();
    store.add_command_at(
        run_id,
        &NewCommand {
            command: command.into(),
            decision: decision.to_string(),
            exit_code,
            stdout_path: None,
            stderr_path: None,
        },
        &at,
    )?;
    Ok(())
}

/// Append a file-change row (the demo's stand-in for "the final git diff").
fn file_change(
    store: &Store,
    run_id: &str,
    clock: &mut Clock,
    path: impl Into<String>,
    change_type: ChangeType,
    diff_summary: impl Into<String>,
) -> Result<()> {
    let at = clock.tick();
    store.add_file_change_at(
        run_id,
        &NewFileChange {
            path: path.into(),
            change_type: change_type.as_str().to_string(),
            diff_summary: Some(diff_summary.into()),
        },
        &at,
    )?;
    Ok(())
}

/// Append a test-result row.
fn test_result(
    store: &Store,
    run_id: &str,
    clock: &mut Clock,
    command: impl Into<String>,
    passed: bool,
    output_summary: impl Into<String>,
) -> Result<()> {
    let at = clock.tick();
    store.add_test_result_at(
        run_id,
        &NewTestResult {
            command: command.into(),
            status: if passed { "passed" } else { "failed" }.to_string(),
            output_summary: Some(output_summary.into()),
        },
        &at,
    )?;
    Ok(())
}

// --- Scenario: normal_feature ------------------------------------------------

fn normal_feature(store: &Store, project_id: &str, seed: u64) -> Result<Run> {
    const NAME: &str = "normal_feature";
    let mut clock = Clock::new(seed);

    let command = "claude add input validation to the signup form".to_string();
    let started_at = clock.tick();
    let run = store.create_run_at(
        &NewRun {
            project_id: project_id.to_string(),
            command: command.clone(),
            agent_name: Some("claude-code".to_string()),
            user_prompt: Some(
                "Add input validation to the signup form and cover the empty-email case with a test."
                    .to_string(),
            ),
            starting_commit: Some("a1b2c3d".to_string()),
        },
        &started_at,
    )?;

    note_synthetic(store, &run.id, NAME, &mut clock)?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunCreated,
        format!("Run created for `{command}`"),
        None,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionStarted,
        "Session started (claude-code)",
        None,
    )?;

    cmd(
        store,
        &run.id,
        &mut clock,
        "rg \"signup\" src/",
        "allow",
        Some(0),
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FileOpened,
        "Opened src/components/SignupForm.tsx",
        Some(json!({"path": "src/components/SignupForm.tsx"})),
    )?;
    cmd(
        store,
        &run.id,
        &mut clock,
        "cat src/components/SignupForm.tsx",
        "allow",
        Some(0),
    )?;

    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FileModified,
        "Modified src/components/SignupForm.tsx",
        Some(json!({"path": "src/components/SignupForm.tsx"})),
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FileModified,
        "Modified src/components/SignupForm.test.tsx",
        Some(json!({"path": "src/components/SignupForm.test.tsx"})),
    )?;

    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::BuildStarted,
        "Check started: npm test -- signup",
        None,
    )?;
    cmd(
        store,
        &run.id,
        &mut clock,
        "npm test -- signup",
        "allow",
        Some(0),
    )?;
    test_result(
        store,
        &run.id,
        &mut clock,
        "npm test -- signup",
        true,
        "PASS src/components/SignupForm.test.tsx\n  \u{2713} rejects empty email (4 ms)\n  \u{2713} accepts a valid email (2 ms)\n\nTests: 2 passed, 2 total",
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::TestsPassed,
        "Check passed: npm test -- signup",
        None,
    )?;

    // Final diff = source of truth for file changes, recorded once, right
    // before the FinalDiffCaptured event — same order as a real `trc run`.
    file_change(
        store,
        &run.id,
        &mut clock,
        "src/components/SignupForm.tsx",
        ChangeType::Modified,
        "+18 -3",
    )?;
    file_change(
        store,
        &run.id,
        &mut clock,
        "src/components/SignupForm.test.tsx",
        ChangeType::Modified,
        "+24 -0",
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FinalDiffCaptured,
        "Captured final diff: 2 files changed",
        None,
    )?;

    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionEnded,
        "Session ended (claude-code)",
        None,
    )?;

    let ended_at = clock.tick();
    store.finish_run_at(
        &run.id,
        RunStatus::Completed,
        Some(0),
        Some("d4e5f6a"),
        &ended_at,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunCompleted,
        "Run completed (exit 0)",
        None,
    )?;

    Ok(run)
}

// --- Scenario: failed_test_repair --------------------------------------------

fn failed_test_repair(store: &Store, project_id: &str, seed: u64) -> Result<Run> {
    const NAME: &str = "failed_test_repair";
    let mut clock = Clock::new(seed);

    let command = "claude fix the flaky checkout total calculation".to_string();
    let started_at = clock.tick();
    let run = store.create_run_at(
        &NewRun {
            project_id: project_id.to_string(),
            command: command.clone(),
            agent_name: Some("claude-code".to_string()),
            user_prompt: Some(
                "The checkout total is off by a cent on discounted orders. Find and fix it."
                    .to_string(),
            ),
            starting_commit: Some("b7c8d9e".to_string()),
        },
        &started_at,
    )?;

    note_synthetic(store, &run.id, NAME, &mut clock)?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunCreated,
        format!("Run created for `{command}`"),
        None,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionStarted,
        "Session started (claude-code)",
        None,
    )?;
    cmd(
        store,
        &run.id,
        &mut clock,
        "cat src/checkout/total.ts",
        "allow",
        Some(0),
    )?;

    // Attempt 1: edit, test — fails.
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FileModified,
        "Modified src/checkout/total.ts",
        Some(json!({"path": "src/checkout/total.ts"})),
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::BuildStarted,
        "Check started: npm test -- checkout/total",
        None,
    )?;
    cmd(
        store,
        &run.id,
        &mut clock,
        "npm test -- checkout/total",
        "allow",
        Some(1),
    )?;
    test_result(
        store,
        &run.id,
        &mut clock,
        "npm test -- checkout/total",
        false,
        "FAIL src/checkout/total.test.ts\n  \u{2717} rounds discounted totals to the nearest cent (11 ms)\n\n    Expected: 19.99\n    Received: 19.98\n\nTests: 1 failed, 1 total",
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::TestsFailed,
        "Check failed: npm test -- checkout/total",
        None,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::Note,
        "Diagnosis: rounding is applied before the discount instead of after",
        None,
    )?;

    // Attempt 2: edit again, test — passes.
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FileModified,
        "Modified src/checkout/total.ts",
        Some(json!({"path": "src/checkout/total.ts"})),
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::BuildStarted,
        "Check started: npm test -- checkout/total",
        None,
    )?;
    cmd(
        store,
        &run.id,
        &mut clock,
        "npm test -- checkout/total",
        "allow",
        Some(0),
    )?;
    test_result(
        store,
        &run.id,
        &mut clock,
        "npm test -- checkout/total",
        true,
        "PASS src/checkout/total.test.ts\n  \u{2713} rounds discounted totals to the nearest cent (9 ms)\n\nTests: 1 passed, 1 total",
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::TestsPassed,
        "Check passed: npm test -- checkout/total",
        None,
    )?;

    file_change(
        store,
        &run.id,
        &mut clock,
        "src/checkout/total.ts",
        ChangeType::Modified,
        "+7 -5",
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::FinalDiffCaptured,
        "Captured final diff: 1 file changed",
        None,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionEnded,
        "Session ended (claude-code)",
        None,
    )?;

    let ended_at = clock.tick();
    store.finish_run_at(
        &run.id,
        RunStatus::Completed,
        Some(0),
        Some("e1f2a3b"),
        &ended_at,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunCompleted,
        "Run completed (exit 0)",
        None,
    )?;

    Ok(run)
}

// --- Scenario: dangerous_command ---------------------------------------------

fn dangerous_command(store: &Store, project_id: &str, seed: u64) -> Result<Run> {
    const NAME: &str = "dangerous_command";
    let mut clock = Clock::new(seed);

    let command = "claude free up disk space by cleaning build artifacts".to_string();
    let started_at = clock.tick();
    let run = store.create_run_at(
        &NewRun {
            project_id: project_id.to_string(),
            command: command.clone(),
            agent_name: Some("claude-code".to_string()),
            user_prompt: Some(
                "The disk is almost full. Clean up old build artifacts and caches.".to_string(),
            ),
            starting_commit: Some("f1e2d3c".to_string()),
        },
        &started_at,
    )?;

    note_synthetic(store, &run.id, NAME, &mut clock)?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunCreated,
        format!("Run created for `{command}`"),
        None,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionStarted,
        "Session started (claude-code)",
        None,
    )?;
    cmd(store, &run.id, &mut clock, "du -sh ./*", "allow", Some(0))?;

    // The agent overreaches: it tries to wipe the whole filesystem instead of
    // scoping the delete. Run this through the REAL command guard — this is
    // the one call in the whole module that is not a canned string, so the
    // scenario proves a real `command_guard` finding, not a decorative one.
    let dangerous = "rm -rf /";
    let guard_result = guard::classify(dangerous);
    anyhow::ensure!(
        guard_result.decision == guard::Decision::Block,
        "demo scenario invariant broken: `{dangerous}` must classify as Block \
         (guard rules changed underneath `trc demo dangerous_command`)"
    );

    let at = clock.tick();
    store.add_command_at(
        &run.id,
        &NewCommand {
            command: dangerous.to_string(),
            decision: guard_result.decision.as_str().to_string(),
            exit_code: None,
            stdout_path: None,
            stderr_path: None,
        },
        &at,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RiskyCommandBlocked,
        format!("Blocked: {}", guard_result.reason),
        Some(json!({"command": dangerous})),
    )?;

    // Mirrors the real blocked-run path in `trc run`: finalize as Blocked and
    // stop — no RunCompleted/RunFailed event, no diff, no session-end. The
    // command never ran.
    let ended_at = clock.tick();
    store.finish_run_at(&run.id, RunStatus::Blocked, None, None, &ended_at)?;

    Ok(run)
}

// --- Scenario: retry_loop ----------------------------------------------------

fn retry_loop(store: &Store, project_id: &str, seed: u64) -> Result<Run> {
    const NAME: &str = "retry_loop";
    let mut clock = Clock::new(seed);

    let command = "claude make the flaky integration test pass".to_string();
    let started_at = clock.tick();
    let run = store.create_run_at(
        &NewRun {
            project_id: project_id.to_string(),
            command: command.clone(),
            agent_name: Some("claude-code".to_string()),
            user_prompt: Some(
                "tests/integration/checkout.spec.ts keeps failing intermittently. Make it pass."
                    .to_string(),
            ),
            starting_commit: Some("9a8b7c6".to_string()),
        },
        &started_at,
    )?;

    note_synthetic(store, &run.id, NAME, &mut clock)?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunCreated,
        format!("Run created for `{command}`"),
        None,
    )?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionStarted,
        "Session started (claude-code)",
        None,
    )?;

    // The exact same command, unchanged, run 4 times in a row without the
    // failure ever being diagnosed differently — the classic "agent is stuck"
    // pattern. `retry_loop_v1` requires >= 3 exact trailing repeats
    // (MIN_REPEATS), so 4 is comfortably over the threshold and proves this
    // demo data actually exercises the real analyzer, not just decorative
    // event text.
    const REPEATS: usize = 4;
    let retried_command = "npm run test:integration -- checkout.spec.ts";
    for i in 0..REPEATS {
        ev(
            store,
            &run.id,
            NAME,
            &mut clock,
            EventType::CommandStarted,
            format!("Started `{retried_command}` (attempt {})", i + 1),
            None,
        )?;
        cmd(
            store,
            &run.id,
            &mut clock,
            retried_command,
            "allow",
            Some(1),
        )?;
        ev(
            store,
            &run.id,
            NAME,
            &mut clock,
            EventType::TestsFailed,
            format!("Check failed: {retried_command} (attempt {})", i + 1),
            None,
        )?;
    }

    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::SessionEnded,
        "Session ended (claude-code)",
        None,
    )?;

    // The run never actually resolved the flake — it gave up after repeating
    // the same failing command. Failed, not Completed: the terminal outcome
    // must not look like a clean success.
    let ended_at = clock.tick();
    store.finish_run_at(&run.id, RunStatus::Failed, Some(1), None, &ended_at)?;
    ev(
        store,
        &run.id,
        NAME,
        &mut clock,
        EventType::RunFailed,
        "Run failed (exit 1)",
        None,
    )?;

    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intel::run_intel_pipeline;

    fn seed_project(store: &Store) -> Project {
        store
            .upsert_project(&NewProject {
                name: "trace-demo-test".into(),
                path: "trace-demo-test".into(),
                config_path: "trace-demo-test/.trace/config.toml".into(),
            })
            .unwrap()
    }

    #[test]
    fn unknown_scenario_is_a_clean_error() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let err = run_scenario(&store, &project.id, "does_not_exist", 1).unwrap_err();
        assert!(err.to_string().contains("unknown demo scenario"));
    }

    #[test]
    fn every_listed_scenario_generates_without_error() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        for s in SCENARIOS {
            let run = run_scenario(&store, &project.id, s.name, 7)
                .unwrap_or_else(|e| panic!("scenario {} failed: {e}", s.name));
            assert_eq!(run.project_id, project.id);
        }
    }

    #[test]
    fn normal_feature_has_the_right_shape() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let run = run_scenario(&store, &project.id, "normal_feature", 1).unwrap();

        let commands = store.list_commands(&run.id).unwrap();
        assert_eq!(commands.len(), 3, "read, cat, npm test");

        let file_changes = store.list_file_changes(&run.id).unwrap();
        assert_eq!(file_changes.len(), 2);

        let test_results = store.list_test_results(&run.id).unwrap();
        assert_eq!(test_results.len(), 1);
        assert_eq!(test_results[0].status, "passed");

        let stored = store.run_by_id(&run.id).unwrap().unwrap();
        assert_eq!(RunStatus::from_str(&stored.status), RunStatus::Completed);
        let summary = store.run_summary(&stored).unwrap();
        assert_eq!(summary.outcome, "SUCCESS");
    }

    #[test]
    fn failed_test_repair_has_the_right_shape() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let run = run_scenario(&store, &project.id, "failed_test_repair", 1).unwrap();

        let test_results = store.list_test_results(&run.id).unwrap();
        assert_eq!(test_results.len(), 2);
        assert_eq!(test_results[0].status, "failed");
        assert_eq!(test_results[1].status, "passed");

        let file_changes = store.list_file_changes(&run.id).unwrap();
        assert_eq!(file_changes.len(), 1, "one file, final diff only");

        let stored = store.run_by_id(&run.id).unwrap().unwrap();
        assert_eq!(RunStatus::from_str(&stored.status), RunStatus::Completed);
        let summary = store.run_summary(&stored).unwrap();
        // `checks_status` reflects the MOST RECENT check only (see
        // `Store::run_summary`), and the repair's last test run passed — so
        // this correctly reads as a clean SUCCESS, not a warning, even though
        // an earlier attempt in the run's own history failed.
        assert_eq!(summary.outcome, "SUCCESS");
    }

    #[test]
    fn dangerous_command_is_blocked_via_the_real_guard() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let run = run_scenario(&store, &project.id, "dangerous_command", 1).unwrap();

        let commands = store.list_commands(&run.id).unwrap();
        assert!(commands.iter().any(|c| c.decision == "block"));

        let stored = store.run_by_id(&run.id).unwrap().unwrap();
        assert_eq!(RunStatus::from_str(&stored.status), RunStatus::Blocked);
        let summary = store.run_summary(&stored).unwrap();
        assert_eq!(summary.outcome, "BLOCKED");

        // The blocked command must be the REAL guard's own text, not a
        // decorative stand-in — a hand-authored "reason" here would defeat
        // the point of proving this exercises `guard::classify`.
        let real = guard::classify("rm -rf /");
        let events = store.list_events(&run.id).unwrap();
        assert!(events
            .iter()
            .any(|e| e.event_type == "risky_command_blocked" && e.message.contains(&real.reason)));
    }

    #[test]
    fn retry_loop_scenario_has_four_identical_trailing_commands() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let run = run_scenario(&store, &project.id, "retry_loop", 1).unwrap();

        let commands = store.list_commands(&run.id).unwrap();
        assert_eq!(commands.len(), 4);
        let sig = "npm run test:integration -- checkout.spec.ts";
        assert!(commands.iter().all(|c| c.command == sig));

        let stored = store.run_by_id(&run.id).unwrap().unwrap();
        assert_eq!(RunStatus::from_str(&stored.status), RunStatus::Failed);
    }

    /// The load-bearing test: the retry_loop scenario's generated data, run
    /// through the REAL deterministic intel pipeline (mapper + analyzers,
    /// unmodified), actually produces a `retry_loop_v1` signal. Proves the
    /// demo data is realistic enough to exercise real analyzers, not just
    /// decorative event text.
    #[test]
    fn retry_loop_scenario_trips_the_real_retry_loop_analyzer() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let run = run_scenario(&store, &project.id, "retry_loop", 3).unwrap();

        let bundle = run_intel_pipeline(&store, &run.id).unwrap().unwrap();
        let retry_signals: Vec<_> = bundle
            .signals
            .iter()
            .filter(|s| s.algorithm_id == "retry_loop_v1")
            .collect();
        assert_eq!(
            retry_signals.len(),
            1,
            "expected exactly one retry_loop_v1 signal, got: {:?}",
            bundle.signals
        );
        assert_eq!(retry_signals[0].kind, "retry_loop");
        assert_eq!(retry_signals[0].observed, Some(serde_json::json!(4)));
    }

    /// The other three scenarios must NOT spuriously trip the retry-loop
    /// analyzer — they resemble real work, not a stuck agent.
    #[test]
    fn non_retry_scenarios_never_trip_the_retry_loop_analyzer() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        for name in ["normal_feature", "failed_test_repair", "dangerous_command"] {
            let run = run_scenario(&store, &project.id, name, 5).unwrap();
            if let Some(bundle) = run_intel_pipeline(&store, &run.id).unwrap() {
                assert!(
                    bundle
                        .signals
                        .iter()
                        .all(|s| s.algorithm_id != "retry_loop_v1"),
                    "scenario {name} unexpectedly tripped retry_loop_v1"
                );
            }
        }
    }

    #[test]
    fn every_row_is_tagged_synthetic() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        for s in SCENARIOS {
            let run = run_scenario(&store, &project.id, s.name, 2).unwrap();
            let events = store.list_events(&run.id).unwrap();
            assert!(!events.is_empty(), "{} produced no events", s.name);
            for e in &events {
                let meta = e.metadata_json.as_deref().unwrap_or_default();
                assert!(
                    meta.contains("\"synthetic\":true"),
                    "{}: event {} missing synthetic tag, metadata: {meta:?}",
                    s.name,
                    e.event_type
                );
                assert!(meta.contains(&format!("\"scenario\":\"{}\"", s.name)));
            }
        }
    }

    #[test]
    fn same_seed_same_sequence_different_seeds_still_same_shape() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        for name in ["normal_feature", "failed_test_repair", "retry_loop"] {
            let run_a = run_scenario(&store, &project.id, name, 42).unwrap();
            let run_b = run_scenario(&store, &project.id, name, 42).unwrap();
            let run_c = run_scenario(&store, &project.id, name, 999).unwrap();

            let cmds_a: Vec<String> = store
                .list_commands(&run_a.id)
                .unwrap()
                .into_iter()
                .map(|c| c.command)
                .collect();
            let cmds_b: Vec<String> = store
                .list_commands(&run_b.id)
                .unwrap()
                .into_iter()
                .map(|c| c.command)
                .collect();
            let cmds_c: Vec<String> = store
                .list_commands(&run_c.id)
                .unwrap()
                .into_iter()
                .map(|c| c.command)
                .collect();

            // Event/command *sequence* is fixed regardless of seed.
            assert_eq!(cmds_a, cmds_b);
            assert_eq!(cmds_a, cmds_c);
            assert_eq!(cmds_a.len(), cmds_c.len());
        }
    }

    #[test]
    fn timestamps_are_staggered_not_identical() {
        let store = Store::open_in_memory().unwrap();
        let project = seed_project(&store);
        let run = run_scenario(&store, &project.id, "normal_feature", 1).unwrap();
        let events = store.list_events(&run.id).unwrap();
        let unique_timestamps: std::collections::BTreeSet<&str> =
            events.iter().map(|e| e.created_at.as_str()).collect();
        assert!(
            unique_timestamps.len() > 1,
            "all events landed on the same timestamp: {unique_timestamps:?}"
        );
        // Chronological order should already hold from insertion (created_at
        // strictly increases as the clock ticks forward).
        let mut sorted = events.clone();
        sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        assert_eq!(
            events.iter().map(|e| &e.id).collect::<Vec<_>>(),
            sorted.iter().map(|e| &e.id).collect::<Vec<_>>()
        );
    }
}
