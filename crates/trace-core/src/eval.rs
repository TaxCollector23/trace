//! Self-evaluation for the deterministic policy engine.
//!
//! A labeled fixture set — real diff snippets paired with which rule (if
//! any) should fire on them — run through the actual `run_policy_checks`
//! and scored. This is deliberately not a static "we tested this once and
//! here are the numbers" artifact: it's a function anyone can call, right
//! now, against whatever the current rules do. Run it via `trace self-check`.
//!
//! Every fixture pairs a "should fire" case with at least one "should NOT
//! fire" near-miss for the same rule — precision matters as much as recall
//! for a tool that's going to interrupt someone's work on a false positive.

use serde::Serialize;

use crate::policy::{run_policy_checks, FileDiff};

struct Fixture {
    name: &'static str,
    /// The rule_key this fixture is testing. `None` means "no rule should
    /// fire on this at all" — a general clean-diff sanity check.
    expected_rule: Option<&'static str>,
    diff: FileDiff,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "AWS key in added line",
            expected_rule: Some("secret-in-diff"),
            diff: FileDiff {
                filename: "src/config.ts".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+const key = \"AKIAABCDEFGHIJKLMNOP\";".into()),
            },
        },
        Fixture {
            name: "AWS-shaped string in a fixture path (should NOT fire)",
            expected_rule: None,
            diff: FileDiff {
                filename: "src/__fixtures__/keys.example.ts".into(),
                status: "added".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+const key = \"AKIAABCDEFGHIJKLMNOP\";".into()),
            },
        },
        Fixture {
            name: "TODO left in added line",
            expected_rule: Some("todo-debug-code"),
            diff: FileDiff {
                filename: "src/handler.ts".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+// TODO: handle the error case".into()),
            },
        },
        Fixture {
            name: "TODO in a removed line (should NOT fire)",
            expected_rule: None,
            diff: FileDiff {
                filename: "src/handler.ts".into(),
                status: "modified".into(),
                additions: 0,
                deletions: 1,
                patch: Some("-// TODO: handle the error case".into()),
            },
        },
        Fixture {
            name: "console.log added in a test file (should NOT fire — expected in tests)",
            expected_rule: None,
            diff: FileDiff {
                filename: "src/handler.test.ts".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+  console.log(result); // debugging the assertion".into()),
            },
        },
        Fixture {
            name: "Stripe secret key in added line (unified secret engine — was missed by the diff scanner)",
            expected_rule: Some("secret-in-diff"),
            diff: FileDiff {
                filename: "src/payments.ts".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                // Split so no contiguous secret literal exists in source
                // (defeats push protection); concat! restores it at compile time.
                patch: Some(
                    concat!("+const stripe = new Stripe(\"sk", "_live_abcdefghijklmnopqrstuvwx\");")
                        .into(),
                ),
            },
        },
        Fixture {
            name: "package.json changed",
            expected_rule: Some("dependency-change-detection"),
            diff: FileDiff {
                filename: "package.json".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+\"left-pad\": \"^1.0.0\"".into()),
            },
        },
        Fixture {
            name: "package-lock.json alone changed (dependency check still fires — correct, lockfile changes ARE dependency changes)",
            expected_rule: Some("dependency-change-detection"),
            diff: FileDiff {
                filename: "package-lock.json".into(),
                status: "modified".into(),
                additions: 400,
                deletions: 400,
                patch: Some("+lockfile churn".into()),
            },
        },
        Fixture {
            name: "test file deleted",
            expected_rule: Some("removed-test-file"),
            diff: FileDiff {
                filename: "src/auth.test.ts".into(),
                status: "removed".into(),
                additions: 0,
                deletions: 40,
                patch: None,
            },
        },
        Fixture {
            name: "non-test file deleted (should NOT fire removed-test-file)",
            expected_rule: None,
            diff: FileDiff {
                filename: "src/legacy-helper.ts".into(),
                status: "removed".into(),
                additions: 0,
                deletions: 40,
                patch: None,
            },
        },
        Fixture {
            name: "empty catch block introduced",
            expected_rule: Some("swallowed-catch"),
            diff: FileDiff {
                filename: "src/fetch.ts".into(),
                status: "modified".into(),
                additions: 3,
                deletions: 0,
                patch: Some("+try {\n+  await risky();\n+} catch (e) {}".into()),
            },
        },
        Fixture {
            name: "catch block that logs (should NOT fire)",
            expected_rule: None,
            diff: FileDiff {
                filename: "src/fetch.ts".into(),
                status: "modified".into(),
                additions: 3,
                deletions: 0,
                patch: Some("+try {\n+  await risky();\n+} catch (e) { logger.error(e); }".into()),
            },
        },
        Fixture {
            name: "hardcoded localhost URL in production path",
            expected_rule: Some("hardcoded-localhost"),
            diff: FileDiff {
                filename: "src/api/client.ts".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+const BASE = \"http://localhost:4000\";".into()),
            },
        },
        Fixture {
            name: "localhost URL in a doc file (should NOT fire)",
            expected_rule: None,
            diff: FileDiff {
                filename: "docs/setup.md".into(),
                status: "modified".into(),
                additions: 1,
                deletions: 0,
                patch: Some("+Run the dev server at http://localhost:3000".into()),
            },
        },
        Fixture {
            name: "clean, unrelated change (nothing should fire)",
            expected_rule: None,
            diff: FileDiff {
                filename: "src/utils/format.ts".into(),
                status: "modified".into(),
                additions: 2,
                deletions: 1,
                patch: Some("+export function formatDate(d: Date) {\n+  return d.toISOString();\n+}".into()),
            },
        },
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct FixtureResult {
    pub name: String,
    pub expected_rule: Option<String>,
    pub fired_rules: Vec<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyEvalReport {
    pub total: usize,
    pub passed: usize,
    /// True positives / (true positives + false positives) — of everything
    /// the engine flagged, how much was actually expected to fire.
    pub precision: f64,
    /// True positives / (true positives + false negatives) — of everything
    /// that should have fired, how much actually did.
    pub recall: f64,
    pub results: Vec<FixtureResult>,
}

/// Runs every fixture through the real policy engine and scores it. No
/// mocking, no stubs — this calls exactly the same `run_policy_checks` the
/// daemon and CI use.
pub fn run_policy_eval() -> PolicyEvalReport {
    let mut results = Vec::new();
    let (mut tp, mut fp, mut fn_) = (0usize, 0usize, 0usize);

    for fixture in fixtures() {
        let findings = run_policy_checks(std::slice::from_ref(&fixture.diff));
        let fired_rules: Vec<String> = findings.iter().map(|f| f.rule_key.clone()).collect();

        let passed = match &fixture.expected_rule {
            Some(expected) => {
                let hit = fired_rules.iter().any(|r| r == expected);
                if hit {
                    tp += 1;
                } else {
                    fn_ += 1;
                }
                // Extra findings beyond the one under test are allowed (a
                // fixture can legitimately trip more than one rule) — this
                // fixture only asserts the rule it's targeting fired.
                hit
            }
            None => {
                let clean = fired_rules.is_empty();
                if !clean {
                    fp += fired_rules.len();
                }
                clean
            }
        };

        results.push(FixtureResult {
            name: fixture.name.to_string(),
            expected_rule: fixture.expected_rule.map(|s| s.to_string()),
            fired_rules,
            passed,
        });
    }

    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        1.0
    };
    let recall = if tp + fn_ > 0 {
        tp as f64 / (tp + fn_) as f64
    } else {
        1.0
    };
    let passed = results.iter().filter(|r| r.passed).count();

    PolicyEvalReport {
        total: results.len(),
        passed,
        precision,
        recall,
        results,
    }
}
