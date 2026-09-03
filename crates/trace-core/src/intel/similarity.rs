//! Deterministic same-project run similarity + behavior diff (Wave 2, Agent 2).
//!
//! Two independent features, both built from facts already in the `Store` —
//! no ML, no LLM, no network call:
//!
//! - [`find_similar_runs`] — "find similar runs like this one": ranks other
//!   runs in the *same project* by a documented weighted score over command
//!   families, touched files/directories, outcome, and duration.
//! - [`compare_runs`] — "run diff": counts (commands/files/tests/approvals/
//!   blocks/duration) for two arbitrary runs, plus a one-line factual
//!   narrative synthesized from the real deltas by a fixed template. The
//!   narrative is `None` whenever the comparison would be misleading (either
//!   run recorded zero commands) rather than inventing a ratio out of noise.
//!
//! ## Scoring, documented
//!
//! [`similarity_score`] is a weighted sum of four `[0.0, 1.0]` components:
//!
//! | component            | weight | how it's computed                                   |
//! |-----------------------|-------:|------------------------------------------------------|
//! | command-family shape  | 0.35   | cosine similarity over per-family command counts      |
//! | file/dir overlap      | 0.35   | Jaccard index over touched file paths + parent dirs    |
//! | outcome match         | 0.15   | 1.0 if [`RunOutcome`](crate::models::RunOutcome) strings are equal, else 0.0 |
//! | duration proximity    | 0.15   | 1.0 same bucket, 0.5 adjacent bucket, else 0.0         |
//!
//! [`MIN_SIMILARITY`] (0.15) is the floor below which a candidate is dropped
//! rather than padded into the result to make the list look fuller than the
//! data supports. [`MIN_COMPARABLE_RUNS`] (2) is the minimum number of other
//! runs in the project required before a similarity search is attempted at
//! all — with fewer, [`find_similar_runs`] returns an empty list, not a
//! fabricated one.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::models::Run;
use crate::Store;

/// A candidate is dropped (not padded into the results) below this score.
pub const MIN_SIMILARITY: f64 = 0.15;
/// Minimum number of *other* runs in the project required before a
/// similarity search is attempted. Below this, results would be either
/// trivially unranked or drawn from a sample too small to mean anything.
pub const MIN_COMPARABLE_RUNS: usize = 2;
/// Default cap on how many similar runs are returned.
pub const DEFAULT_SIMILAR_LIMIT: usize = 5;

const WEIGHT_FAMILY: f64 = 0.35;
const WEIGHT_FILES: f64 = 0.35;
const WEIGHT_OUTCOME: f64 = 0.15;
const WEIGHT_DURATION: f64 = 0.15;

// --- Command families -------------------------------------------------------

/// A coarse, deterministic bucket for a command line. Classification is
/// simple prefix/keyword matching in the same spirit as `guard::classify`
/// (see `crates/trace-core/src/guard.rs`) — normalize, then match fixed
/// string rules. No fourth classifier is invented: this exists purely to
/// group commands for similarity scoring, a different question than the
/// guard's allow/warn/block decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandFamily {
    Git,
    Npm,
    Test,
    Filesystem,
    Other,
}

impl CommandFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandFamily::Git => "git",
            CommandFamily::Npm => "npm",
            CommandFamily::Test => "test",
            CommandFamily::Filesystem => "filesystem",
            CommandFamily::Other => "other",
        }
    }

    /// Classify one command line. Order matters: a test invocation (`npm
    /// test`, `cargo test`, `pytest ...`) is filed under `Test` even though
    /// it would otherwise match the `Npm`/`Other` package-manager prefix,
    /// because "ran the test suite" is the more useful similarity signal
    /// than "used npm".
    pub fn classify(command: &str) -> CommandFamily {
        let normalized = command.trim().to_lowercase();
        if normalized.is_empty() {
            return CommandFamily::Other;
        }
        const TEST_MARKERS: &[&str] = &[
            "test", "pytest", "jest", "mocha", "rspec", "phpunit", "vitest", "ginkgo",
        ];
        if TEST_MARKERS.iter().any(|m| normalized.contains(m)) {
            return CommandFamily::Test;
        }

        let first = normalized.split_whitespace().next().unwrap_or("");
        if first == "git" {
            return CommandFamily::Git;
        }
        const PKG_MANAGERS: &[&str] = &[
            "npm", "pnpm", "yarn", "npx", "pip", "pip3", "cargo", "go", "bundle", "composer", "gem",
        ];
        if PKG_MANAGERS.contains(&first) {
            return CommandFamily::Npm;
        }
        const FS_COMMANDS: &[&str] = &[
            "rm", "mv", "cp", "mkdir", "touch", "chmod", "chown", "find", "rmdir", "ln",
        ];
        if FS_COMMANDS.contains(&first) {
            return CommandFamily::Filesystem;
        }
        CommandFamily::Other
    }
}

// --- Duration buckets --------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DurationBucket {
    /// Duration could not be computed (run still in progress, or malformed
    /// timestamps). Never treated as "close" to anything, including itself.
    Unknown,
    UnderOneMinute,
    OneToFiveMinutes,
    FiveToFifteenMinutes,
    FifteenToSixtyMinutes,
    OverOneHour,
}

impl DurationBucket {
    fn from_seconds(secs: Option<i64>) -> Self {
        match secs {
            None => DurationBucket::Unknown,
            Some(s) if s < 60 => DurationBucket::UnderOneMinute,
            Some(s) if s < 300 => DurationBucket::OneToFiveMinutes,
            Some(s) if s < 900 => DurationBucket::FiveToFifteenMinutes,
            Some(s) if s < 3600 => DurationBucket::FifteenToSixtyMinutes,
            Some(_) => DurationBucket::OverOneHour,
        }
    }

    /// Ordinal position for adjacency scoring. `None` for `Unknown`, which
    /// has no meaningful distance to any bucket.
    fn ordinal(self) -> Option<u8> {
        match self {
            DurationBucket::Unknown => None,
            DurationBucket::UnderOneMinute => Some(0),
            DurationBucket::OneToFiveMinutes => Some(1),
            DurationBucket::FiveToFifteenMinutes => Some(2),
            DurationBucket::FifteenToSixtyMinutes => Some(3),
            DurationBucket::OverOneHour => Some(4),
        }
    }
}

fn duration_similarity(a: DurationBucket, b: DurationBucket) -> f64 {
    match (a.ordinal(), b.ordinal()) {
        (Some(x), Some(y)) if x == y => 1.0,
        (Some(x), Some(y)) if x.abs_diff(y) == 1 => 0.5,
        _ => 0.0,
    }
}

/// Parse `started_at`/`ended_at` (RFC 3339) into whole-second duration.
/// `None` when the run has not finished or a timestamp fails to parse —
/// never a guessed value.
fn compute_duration_seconds(started_at: &str, ended_at: Option<&str>) -> Option<i64> {
    let ended_at = ended_at?;
    let start = chrono::DateTime::parse_from_rfc3339(started_at).ok()?;
    let end = chrono::DateTime::parse_from_rfc3339(ended_at).ok()?;
    let secs = (end - start).num_seconds();
    if secs < 0 {
        None
    } else {
        Some(secs)
    }
}

// --- Fingerprint --------------------------------------------------------------

/// The deterministic "shape" of a run, built entirely from rows already in
/// the `Store` (commands, file changes, the run row itself). Nothing here is
/// inferred beyond simple counting/classification.
#[derive(Debug, Clone)]
struct RunFingerprint {
    outcome: String,
    duration_seconds: Option<i64>,
    duration_bucket: DurationBucket,
    family_counts: BTreeMap<CommandFamily, u32>,
    /// Touched file paths, plus their parent directories prefixed `dir:` so
    /// two runs that touch different files in the same area of the codebase
    /// still register overlap, not just byte-for-byte identical paths.
    files_touched: BTreeSet<String>,
}

fn build_fingerprint(store: &Store, run: &Run) -> Result<RunFingerprint> {
    let commands = store.list_commands(&run.id)?;
    let mut family_counts: BTreeMap<CommandFamily, u32> = BTreeMap::new();
    for c in &commands {
        *family_counts
            .entry(CommandFamily::classify(&c.command))
            .or_insert(0) += 1;
    }

    let files = store.list_file_changes(&run.id)?;
    let mut files_touched = BTreeSet::new();
    for f in &files {
        files_touched.insert(f.path.clone());
        if let Some(parent) = std::path::Path::new(&f.path).parent() {
            let dir = parent.to_string_lossy();
            if !dir.is_empty() {
                files_touched.insert(format!("dir:{dir}"));
            }
        }
    }

    let outcome = store.run_summary(run)?.outcome;
    let duration_seconds = compute_duration_seconds(&run.started_at, run.ended_at.as_deref());

    Ok(RunFingerprint {
        outcome,
        duration_seconds,
        duration_bucket: DurationBucket::from_seconds(duration_seconds),
        family_counts,
        files_touched,
    })
}

fn family_cosine_similarity(
    a: &BTreeMap<CommandFamily, u32>,
    b: &BTreeMap<CommandFamily, u32>,
) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let families: BTreeSet<&CommandFamily> = a.keys().chain(b.keys()).collect();
    let (mut dot, mut norm_a, mut norm_b) = (0.0_f64, 0.0_f64, 0.0_f64);
    for family in families {
        let x = *a.get(family).unwrap_or(&0) as f64;
        let y = *b.get(family).unwrap_or(&0) as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// The documented weighted score between two fingerprints. See the module
/// docs for the weight table.
fn similarity_score(a: &RunFingerprint, b: &RunFingerprint) -> f64 {
    let family = family_cosine_similarity(&a.family_counts, &b.family_counts);
    let files = jaccard(&a.files_touched, &b.files_touched);
    let outcome = if a.outcome == b.outcome { 1.0 } else { 0.0 };
    let duration = duration_similarity(a.duration_bucket, b.duration_bucket);
    WEIGHT_FAMILY * family
        + WEIGHT_FILES * files
        + WEIGHT_OUTCOME * outcome
        + WEIGHT_DURATION * duration
}

// --- Public: similar runs ----------------------------------------------------

/// One ranked result from [`find_similar_runs`]. Field names/shapes match
/// the `GET /api/runs/:id/similar` wire contract exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarRun {
    pub run_id: String,
    /// `[0.0, 1.0]`; never below [`MIN_SIMILARITY`] — see module docs.
    pub similarity: f64,
    pub command: String,
    pub outcome: String,
    pub duration_seconds: Option<i64>,
    pub started_at: String,
}

/// Rank other runs in the *same project* as `run_id` by [`similarity_score`].
///
/// Returns `Ok(None)` when `run_id` itself does not exist (the caller's cue
/// to answer 404, matching `run_intel_pipeline`'s convention). Returns
/// `Ok(Some(vec![]))` — an honest empty list, not a fabricated one — when:
/// - fewer than [`MIN_COMPARABLE_RUNS`] other runs exist in the project, or
/// - no candidate scores at or above [`MIN_SIMILARITY`].
///
/// Never crosses projects: candidates are drawn only from
/// `Store::list_runs_for_project(target.project_id, ...)`.
pub fn find_similar_runs(
    store: &Store,
    run_id: &str,
    limit: usize,
) -> Result<Option<Vec<SimilarRun>>> {
    let Some(target) = store.run_by_id(run_id)? else {
        return Ok(None);
    };
    let target_fp = build_fingerprint(store, &target)?;

    // A generous pool so the ranking has real candidates to work with; the
    // project scoping (not this limit) is what keeps the search honest.
    let pool = store.list_runs_for_project(&target.project_id, 500)?;
    let others: Vec<Run> = pool.into_iter().filter(|r| r.id != target.id).collect();
    if others.len() < MIN_COMPARABLE_RUNS {
        return Ok(Some(Vec::new()));
    }

    let mut scored = Vec::with_capacity(others.len());
    for candidate in &others {
        let fp = build_fingerprint(store, candidate)?;
        let score = similarity_score(&target_fp, &fp);
        if score >= MIN_SIMILARITY {
            scored.push(SimilarRun {
                run_id: candidate.id.clone(),
                similarity: score,
                command: candidate.command.clone(),
                outcome: fp.outcome,
                duration_seconds: fp.duration_seconds,
                started_at: candidate.started_at.clone(),
            });
        }
    }

    scored.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    Ok(Some(scored))
}

// --- Public: run comparison / behavior diff ----------------------------------

/// Execution-behavior counts for one run — never a git diff. Field names
/// match the `GET /api/runs/compare` wire contract exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCounts {
    pub run_id: String,
    pub command: String,
    pub commands: i64,
    pub files: i64,
    pub test_cycles: i64,
    /// Commands the guard flagged `warn` or `require_approval`.
    pub approvals: i64,
    /// Commands the guard hard-`block`ed.
    pub blocks: i64,
    pub duration_seconds: Option<i64>,
    pub outcome: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunComparison {
    pub run_a: RunCounts,
    pub run_b: RunCounts,
    /// A one-line, template-generated sentence over the real deltas above.
    /// `None` when the comparison would be misleading (see
    /// [`build_narrative`]) — the frontend renders an honest "not enough
    /// data" message instead of a fabricated ratio.
    pub narrative: Option<String>,
}

fn build_counts(store: &Store, run: &Run) -> Result<RunCounts> {
    let commands = store.list_commands(&run.id)?;
    let files = store.list_file_changes(&run.id)?;
    let tests = store.list_test_results(&run.id)?;
    let approvals = commands
        .iter()
        .filter(|c| c.decision == "warn" || c.decision == "require_approval")
        .count() as i64;
    let blocks = commands
        .iter()
        .filter(|c| c.decision == "block" || c.decision == "blocked")
        .count() as i64;
    let outcome = store.run_summary(run)?.outcome;

    Ok(RunCounts {
        run_id: run.id.clone(),
        command: run.command.clone(),
        commands: commands.len() as i64,
        files: files.len() as i64,
        test_cycles: tests.len() as i64,
        approvals,
        blocks,
        duration_seconds: compute_duration_seconds(&run.started_at, run.ended_at.as_deref()),
        outcome,
        started_at: run.started_at.clone(),
    })
}

/// Synthesize the one-line comparison from real command counts, via a fixed
/// template — never an LLM call, never an invented number.
///
/// Returns `None` when either run recorded zero commands: a ratio against
/// zero (or against a run with no meaningful activity) would misrepresent
/// the comparison rather than inform it, so the honest answer is "not
/// enough data", not a number.
fn build_narrative(a: &RunCounts, b: &RunCounts) -> Option<String> {
    if a.commands == 0 || b.commands == 0 {
        return None;
    }
    if a.commands == b.commands {
        return Some(format!(
            "Run A and Run B executed the same number of commands ({} each).",
            a.commands
        ));
    }
    let (more, more_label, fewer, fewer_label) = if a.commands > b.commands {
        (a.commands, "Run A", b.commands, "Run B")
    } else {
        (b.commands, "Run B", a.commands, "Run A")
    };
    let ratio = more as f64 / fewer as f64;
    Some(format!(
        "{more_label} required {ratio:.1}x more command execution than {fewer_label} ({more} vs {fewer} commands)."
    ))
}

/// Compare execution behavior between two arbitrary runs (not restricted to
/// the same project — a deliberate diff of two runs the user picked).
/// Returns `Ok(None)` when either run id does not exist (404 for the
/// caller), never a partial/fabricated comparison.
pub fn compare_runs(
    store: &Store,
    run_a_id: &str,
    run_b_id: &str,
) -> Result<Option<RunComparison>> {
    let (Some(run_a), Some(run_b)) = (store.run_by_id(run_a_id)?, store.run_by_id(run_b_id)?)
    else {
        return Ok(None);
    };
    let counts_a = build_counts(store, &run_a)?;
    let counts_b = build_counts(store, &run_b)?;
    let narrative = build_narrative(&counts_a, &counts_b);
    Ok(Some(RunComparison {
        run_a: counts_a,
        run_b: counts_b,
        narrative,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewCommand, NewFileChange, NewProject, NewRun, NewTestResult};

    fn project(store: &Store, name: &str) -> String {
        store
            .upsert_project(&NewProject {
                name: name.into(),
                path: format!("/tmp/similarity-test-{name}"),
                config_path: format!("/tmp/similarity-test-{name}/.trace/config.toml"),
            })
            .unwrap()
            .id
    }

    fn run_at(store: &Store, project_id: &str, at: &str) -> Run {
        store
            .create_run_at(
                &NewRun {
                    project_id: project_id.into(),
                    command: "run".into(),
                    agent_name: Some("claude-code".into()),
                    user_prompt: None,
                    starting_commit: None,
                },
                at,
            )
            .unwrap()
    }

    fn add_commands(store: &Store, run_id: &str, commands: &[&str]) {
        for c in commands {
            store
                .add_command(
                    run_id,
                    &NewCommand {
                        command: (*c).into(),
                        decision: "allow".into(),
                        exit_code: Some(0),
                        stdout_path: None,
                        stderr_path: None,
                    },
                )
                .unwrap();
        }
    }

    fn add_files(store: &Store, run_id: &str, paths: &[&str]) {
        for p in paths {
            store
                .add_file_change(
                    run_id,
                    &NewFileChange {
                        path: (*p).into(),
                        change_type: "modified".into(),
                        diff_summary: None,
                    },
                )
                .unwrap();
        }
    }

    fn finish(store: &Store, run_id: &str, started_at: &str, secs_later: i64) {
        let start = chrono::DateTime::parse_from_rfc3339(started_at).unwrap();
        let end = start + chrono::Duration::seconds(secs_later);
        store
            .finish_run_at(
                run_id,
                crate::models::RunStatus::Completed,
                Some(0),
                None,
                &end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            )
            .unwrap();
    }

    #[test]
    fn command_family_classification_matches_expected_buckets() {
        assert_eq!(CommandFamily::classify("git status"), CommandFamily::Git);
        assert_eq!(
            CommandFamily::classify("git commit -m x"),
            CommandFamily::Git
        );
        assert_eq!(CommandFamily::classify("npm install"), CommandFamily::Npm);
        assert_eq!(
            CommandFamily::classify("pnpm add left-pad"),
            CommandFamily::Npm
        );
        // "npm test" reads as a test invocation, not a generic npm command.
        assert_eq!(CommandFamily::classify("npm test"), CommandFamily::Test);
        assert_eq!(
            CommandFamily::classify("cargo test --workspace"),
            CommandFamily::Test
        );
        assert_eq!(
            CommandFamily::classify("pytest -k foo"),
            CommandFamily::Test
        );
        assert_eq!(
            CommandFamily::classify("rm -rf build/"),
            CommandFamily::Filesystem
        );
        assert_eq!(
            CommandFamily::classify("mkdir -p out"),
            CommandFamily::Filesystem
        );
        assert_eq!(
            CommandFamily::classify("curl https://example.com"),
            CommandFamily::Other
        );
    }

    /// Two near-identical runs (same command family shape, same files, same
    /// outcome, same duration bucket) score high — above the floor and
    /// clearly in "similar" territory.
    #[test]
    fn near_identical_runs_score_high() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "near-identical");

        let target = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        add_commands(
            &store,
            &target.id,
            &["git status", "npm install", "npm test", "npm test"],
        );
        add_files(
            &store,
            &target.id,
            &["src/api/routes.ts", "src/api/handler.ts"],
        );
        finish(&store, &target.id, "2026-01-01T10:00:00Z", 120);

        let twin = run_at(&store, &pid, "2026-01-02T10:00:00Z");
        add_commands(
            &store,
            &twin.id,
            &["git status", "npm install", "npm test", "npm test"],
        );
        add_files(
            &store,
            &twin.id,
            &["src/api/routes.ts", "src/api/middleware.ts"],
        );
        finish(&store, &twin.id, "2026-01-02T10:00:00Z", 130);

        // A second, unrelated run so MIN_COMPARABLE_RUNS is satisfied.
        let other = run_at(&store, &pid, "2026-01-03T10:00:00Z");
        add_commands(&store, &other.id, &["curl https://example.com"]);
        finish(&store, &other.id, "2026-01-03T10:00:00Z", 3600 * 5);

        let results = find_similar_runs(&store, &target.id, DEFAULT_SIMILAR_LIMIT)
            .unwrap()
            .unwrap();
        let twin_result = results.iter().find(|r| r.run_id == twin.id);
        assert!(twin_result.is_some(), "twin run should appear in results");
        assert!(
            twin_result.unwrap().similarity > 0.7,
            "expected high similarity, got {}",
            twin_result.unwrap().similarity
        );
    }

    /// A run with a totally different shape (different command families,
    /// disjoint files, different outcome, wildly different duration) scores
    /// low — below the floor, so it does not appear at all.
    #[test]
    fn very_different_runs_score_low_and_are_excluded() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "very-different");

        let target = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        add_commands(&store, &target.id, &["git status", "npm test"]);
        add_files(&store, &target.id, &["src/api/routes.ts"]);
        finish(&store, &target.id, "2026-01-01T10:00:00Z", 30);

        let unrelated = run_at(&store, &pid, "2026-01-02T10:00:00Z");
        add_commands(
            &store,
            &unrelated.id,
            &["rm -rf tmp/", "mkdir out", "touch out/a", "touch out/b"],
        );
        add_files(&store, &unrelated.id, &["infra/terraform/main.tf"]);
        // Long-running + failed, unlike the target's short/completed run.
        let start = chrono::DateTime::parse_from_rfc3339("2026-01-02T10:00:00Z").unwrap();
        let end = start + chrono::Duration::seconds(3600 * 3);
        store
            .finish_run_at(
                &unrelated.id,
                crate::models::RunStatus::Failed,
                Some(1),
                None,
                &end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            )
            .unwrap();

        // A third run purely so MIN_COMPARABLE_RUNS is satisfied.
        let filler = run_at(&store, &pid, "2026-01-03T10:00:00Z");
        add_commands(&store, &filler.id, &["git log"]);
        finish(&store, &filler.id, "2026-01-03T10:00:00Z", 20);

        let results = find_similar_runs(&store, &target.id, DEFAULT_SIMILAR_LIMIT)
            .unwrap()
            .unwrap();
        assert!(
            results.iter().all(|r| r.run_id != unrelated.id),
            "a dissimilar run must not appear in similar results: {results:?}"
        );
    }

    /// Same-project floor: fewer than MIN_COMPARABLE_RUNS other runs in the
    /// project returns an honest empty list, not a padded/fabricated one.
    #[test]
    fn too_few_comparable_runs_returns_empty_not_fabricated() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "too-few");

        let target = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        add_commands(&store, &target.id, &["git status"]);
        finish(&store, &target.id, "2026-01-01T10:00:00Z", 30);

        // Exactly one other run — below MIN_COMPARABLE_RUNS (2).
        let only_other = run_at(&store, &pid, "2026-01-01T11:00:00Z");
        add_commands(&store, &only_other.id, &["git status"]);
        finish(&store, &only_other.id, "2026-01-01T11:00:00Z", 30);

        let results = find_similar_runs(&store, &target.id, DEFAULT_SIMILAR_LIMIT)
            .unwrap()
            .unwrap();
        assert!(results.is_empty());
    }

    /// Similarity search never crosses projects: a near-identical run in a
    /// different project must never appear.
    #[test]
    fn similarity_never_crosses_projects() {
        let store = Store::open_in_memory().unwrap();
        let pid_a = project(&store, "proj-a");
        let pid_b = project(&store, "proj-b");

        let target = run_at(&store, &pid_a, "2026-01-01T10:00:00Z");
        add_commands(&store, &target.id, &["git status", "npm test"]);
        add_files(&store, &target.id, &["src/api/routes.ts"]);
        finish(&store, &target.id, "2026-01-01T10:00:00Z", 60);

        // Two other runs in project A -> satisfies MIN_COMPARABLE_RUNS, so
        // this exercises the candidate-pool query itself (not just the
        // threshold early-return) for cross-project leakage.
        let other_in_a = run_at(&store, &pid_a, "2026-01-01T11:00:00Z");
        add_commands(&store, &other_in_a.id, &["git log"]);
        finish(&store, &other_in_a.id, "2026-01-01T11:00:00Z", 60);
        let other_in_a_2 = run_at(&store, &pid_a, "2026-01-01T12:00:00Z");
        add_commands(&store, &other_in_a_2.id, &["git log"]);
        finish(&store, &other_in_a_2.id, "2026-01-01T12:00:00Z", 60);

        let twin_in_b = run_at(&store, &pid_b, "2026-01-01T10:05:00Z");
        add_commands(&store, &twin_in_b.id, &["git status", "npm test"]);
        add_files(&store, &twin_in_b.id, &["src/api/routes.ts"]);
        finish(&store, &twin_in_b.id, "2026-01-01T10:05:00Z", 60);

        let results = find_similar_runs(&store, &target.id, DEFAULT_SIMILAR_LIMIT)
            .unwrap()
            .unwrap();
        assert!(results.iter().all(|r| r.run_id != twin_in_b.id));
    }

    #[test]
    fn unknown_run_id_returns_none() {
        let store = Store::open_in_memory().unwrap();
        assert!(
            find_similar_runs(&store, "does-not-exist", DEFAULT_SIMILAR_LIMIT)
                .unwrap()
                .is_none()
        );
    }

    /// Same command family, very different volume: the narrative states the
    /// real ratio computed from the counts, not an invented number.
    #[test]
    fn narrative_states_real_ratio_for_same_family_different_volume() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "narrative-ratio");

        let run_a = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        add_commands(&store, &run_a.id, &["git status", "npm test"]);
        finish(&store, &run_a.id, "2026-01-01T10:00:00Z", 60);

        let run_b = run_at(&store, &pid, "2026-01-02T10:00:00Z");
        add_commands(
            &store,
            &run_b.id,
            &["git status", "npm test", "npm test", "npm test", "npm test"],
        );
        // 4 test_results rows = 4 recorded test cycles.
        for _ in 0..4 {
            store
                .add_test_result(
                    &run_b.id,
                    &NewTestResult {
                        command: "npm test".into(),
                        status: "failed".into(),
                        output_summary: None,
                    },
                )
                .unwrap();
        }
        finish(&store, &run_b.id, "2026-01-02T10:00:00Z", 60);

        let cmp = compare_runs(&store, &run_a.id, &run_b.id).unwrap().unwrap();
        assert_eq!(cmp.run_a.commands, 2);
        assert_eq!(cmp.run_b.commands, 5);
        assert_eq!(cmp.run_b.test_cycles, 4);
        let narrative = cmp.narrative.expect("both runs have commands");
        assert!(narrative.contains("Run B"));
        assert!(narrative.contains("2.5x"));
        assert!(narrative.contains("5 vs 2 commands"));
    }

    /// Too little data: a run with zero recorded commands makes the
    /// comparison meaningless, so the narrative is null rather than a
    /// nonsense ratio against zero.
    #[test]
    fn zero_command_run_yields_null_narrative_not_nonsense() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "zero-commands");

        let run_a = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        // No commands recorded for run_a at all.
        finish(&store, &run_a.id, "2026-01-01T10:00:00Z", 10);

        let run_b = run_at(&store, &pid, "2026-01-02T10:00:00Z");
        add_commands(&store, &run_b.id, &["git status", "npm test"]);
        finish(&store, &run_b.id, "2026-01-02T10:00:00Z", 60);

        let cmp = compare_runs(&store, &run_a.id, &run_b.id).unwrap().unwrap();
        assert!(cmp.narrative.is_none());
    }

    #[test]
    fn compare_unknown_run_returns_none() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "compare-unknown");
        let run_a = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        finish(&store, &run_a.id, "2026-01-01T10:00:00Z", 10);

        assert!(compare_runs(&store, &run_a.id, "does-not-exist")
            .unwrap()
            .is_none());
    }

    #[test]
    fn equal_command_counts_produce_an_equality_narrative() {
        let store = Store::open_in_memory().unwrap();
        let pid = project(&store, "equal-counts");
        let run_a = run_at(&store, &pid, "2026-01-01T10:00:00Z");
        add_commands(&store, &run_a.id, &["git status", "npm test"]);
        finish(&store, &run_a.id, "2026-01-01T10:00:00Z", 10);

        let run_b = run_at(&store, &pid, "2026-01-02T10:00:00Z");
        add_commands(&store, &run_b.id, &["git log", "npm install"]);
        finish(&store, &run_b.id, "2026-01-02T10:00:00Z", 10);

        let cmp = compare_runs(&store, &run_a.id, &run_b.id).unwrap().unwrap();
        let narrative = cmp.narrative.expect("both runs have commands");
        assert!(narrative.contains("same number of commands"));
    }
}
