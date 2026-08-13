//! Deterministic policy engine.
//!
//! Ported from Ratify's PR-review policy checks. Runs on any set of file
//! diffs — a local agent run's file changes, or a GitHub PR's changed files
//! in CI mode — so the same rules protect you whether Trace is watching a
//! live coding session or reviewing a pull request. No network calls, no
//! LLM, no API key: pure pattern matching that always runs.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A single changed file, independent of where it came from (local run or PR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub filename: String,
    /// "added" | "modified" | "removed" | "renamed"
    pub status: String,
    pub additions: i64,
    pub deletions: i64,
    /// Unified diff patch text, if available.
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFinding {
    pub rule_key: String,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub severity: Severity,
    /// 0.0-1.0. The policy engine is deterministic, so this reflects how
    /// precise the *pattern* is, not uncertainty about a single instance.
    pub confidence: f64,
    pub source: String,
}

fn finding(
    rule_key: &str,
    title: impl Into<String>,
    description: impl Into<String>,
    file_path: Option<String>,
    severity: Severity,
    confidence: f64,
) -> PolicyFinding {
    PolicyFinding {
        rule_key: rule_key.to_string(),
        title: title.into(),
        description: description.into(),
        file_path,
        severity,
        confidence,
        source: "policy-engine".to_string(),
    }
}

fn added_lines(patch: &str) -> String {
    patch
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .collect::<Vec<_>>()
        .join("\n")
}

static SENSITIVE_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)payment|billing|charge|checkout|invoice").unwrap());
static TEST_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\.(test|spec)\.[jt]sx?$|__tests__/|/tests?/").unwrap());
static DEBUG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\+.*\b(TODO|FIXME|console\.log|debugger)\b").unwrap());
static HANDLER_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"/(api|handlers|routes|controllers)/").unwrap());
static MANIFEST_PATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^package\.json$|^package-lock\.json$|^pnpm-lock\.yaml$|^yarn\.lock$|requirements\.txt$|go\.mod$|go\.sum$|Cargo\.toml$|Cargo\.lock$|Gemfile$|Gemfile\.lock$|composer\.lock$|Pipfile\.lock$|poetry\.lock$",
    )
    .unwrap()
});
static FIXTURE_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\.(example|sample|fixture|template)\b|/fixtures?/").unwrap());
static DOC_OR_TEMPLATE_PATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\.(md|mdx|txt|example|sample|template)$|/(docs?|examples?)/").unwrap()
});
static LOCALHOST_URL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)https?://(localhost|127\.0\.0\.1)(:\d+)?").unwrap());
static SWALLOWED_CATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\+.*\bcatch\s*\([^)]*\)\s*\{\s*(//[^\n]*|/\*[^*]*\*/)?\s*\}").unwrap()
});
static DB_CALL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(prisma|drizzle|knex|pg|mysql2)\.(query|execute|\$queryRaw|\$executeRaw)\b|\bnew Pool\(|\bawait sql`").unwrap()
});
static MIGRATION_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\.(sql|migration\.[jt]s)$").unwrap());
static MIGRATION_DIR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)/(migrations?|drizzle|alembic/versions|prisma/migrations)/").unwrap()
});
static LOCKFILE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"pnpm-lock\.yaml|package-lock\.json|yarn\.lock").unwrap());

fn check_missing_tests(files: &[FileDiff]) -> Option<PolicyFinding> {
    let sensitive: Vec<&FileDiff> = files
        .iter()
        .filter(|f| SENSITIVE_PATH.is_match(&f.filename) && !TEST_PATH.is_match(&f.filename))
        .collect();
    if sensitive.is_empty() {
        return None;
    }
    if files.iter().any(|f| TEST_PATH.is_match(&f.filename)) {
        return None;
    }
    Some(finding(
        "missing-tests-for-payments-paths",
        "Missing tests for payment-sensitive change",
        format!(
            "{} file(s) touching payment/billing logic changed with no corresponding test file in this diff.",
            sensitive.len()
        ),
        Some(sensitive[0].filename.clone()),
        Severity::High,
        0.85,
    ))
}

fn check_debug_code(files: &[FileDiff]) -> Vec<PolicyFinding> {
    files
        .iter()
        .filter(|f| {
            // TODO/console.log/debugger are expected in tests, docs, and
            // templates — flagging them there is noise, exactly where the other
            // content rules already stay quiet.
            !TEST_PATH.is_match(&f.filename) && !DOC_OR_TEMPLATE_PATH.is_match(&f.filename)
        })
        .filter_map(|f| {
            let patch = f.patch.as_deref()?;
            DEBUG_PATTERN.is_match(patch).then(|| {
                finding(
                    "todo-debug-code",
                    "Debug code or TODO left in diff",
                    format!(
                        "{} contains an added line with TODO, FIXME, console.log, or debugger.",
                        f.filename
                    ),
                    Some(f.filename.clone()),
                    Severity::Low,
                    0.9,
                )
            })
        })
        .collect()
}

fn check_dependency_change(files: &[FileDiff]) -> Option<PolicyFinding> {
    let manifests: Vec<&FileDiff> = files
        .iter()
        .filter(|f| MANIFEST_PATH.is_match(&f.filename))
        .collect();
    if manifests.is_empty() {
        return None;
    }
    Some(finding(
        "dependency-change-detection",
        "Dependency manifest changed",
        "This change modifies a dependency manifest. Confirm the new dependency is approved.",
        Some(manifests[0].filename.clone()),
        Severity::Medium,
        0.75,
    ))
}

fn check_secrets(files: &[FileDiff]) -> Vec<PolicyFinding> {
    let mut out = Vec::new();
    for f in files {
        let Some(patch) = f.patch.as_deref() else {
            continue;
        };
        if FIXTURE_PATH.is_match(&f.filename) {
            continue;
        }
        let added = added_lines(patch);
        // Reuse the single, maintained secret scanner rather than a parallel
        // pattern list — otherwise the diff path silently misses providers the
        // prompt/command path catches. De-dup by type so one file repeating a
        // key doesn't emit duplicate findings.
        let mut seen = std::collections::HashSet::new();
        for hit in crate::secrets::scan_text(&added) {
            if !seen.insert(hit.secret_type.clone()) {
                continue;
            }
            let label = hit.secret_type;
            out.push(finding(
                "secret-in-diff",
                format!("Possible {label} committed"),
                format!(
                    "An added line in {} matches the pattern for a {label}. If it's real, rotate immediately and remove from the diff. If it's a fixture, move it under a fixtures/ path.",
                    f.filename
                ),
                Some(f.filename.clone()),
                Severity::High,
                0.9,
            ));
        }
    }
    out
}

fn check_removed_tests(files: &[FileDiff]) -> Vec<PolicyFinding> {
    files
        .iter()
        .filter(|f| f.status == "removed" && TEST_PATH.is_match(&f.filename))
        .map(|f| {
            finding(
                "removed-test-file",
                "Test file deleted",
                format!("{} was removed. Confirm the covered behavior is either gone or exercised by another test.", f.filename),
                Some(f.filename.clone()),
                Severity::Medium,
                0.8,
            )
        })
        .collect()
}

fn check_large_file_change(files: &[FileDiff]) -> Vec<PolicyFinding> {
    files
        .iter()
        .filter(|f| f.additions + f.deletions > 500 && !LOCKFILE.is_match(&f.filename))
        .map(|f| {
            finding(
                "large-single-file-change",
                "Large single-file change",
                format!(
                    "{} changed {} lines in one file. Consider splitting into smaller changes or calling out the intent explicitly.",
                    f.filename,
                    f.additions + f.deletions
                ),
                Some(f.filename.clone()),
                Severity::Low,
                0.7,
            )
        })
        .collect()
}

fn check_direct_db_access_in_handler(files: &[FileDiff]) -> Vec<PolicyFinding> {
    let mut out = Vec::new();
    for f in files {
        let Some(patch) = f.patch.as_deref() else {
            continue;
        };
        if !HANDLER_PATH.is_match(&f.filename) {
            continue;
        }
        let added = added_lines(patch);
        if DB_CALL.is_match(&added) {
            out.push(finding(
                "direct-db-access-in-handler",
                "Handler talks to the database directly",
                format!(
                    "{} lives on an HTTP handler path and issues a raw database call in this diff. Most repos abstract this behind a service/repository layer — confirm this is intentional.",
                    f.filename
                ),
                Some(f.filename.clone()),
                Severity::Medium,
                0.65,
            ));
        }
    }
    out
}

fn check_migration_added(files: &[FileDiff]) -> Vec<PolicyFinding> {
    let migrations: Vec<&FileDiff> = files
        .iter()
        .filter(|f| {
            f.status == "added"
                && MIGRATION_PATH.is_match(&f.filename)
                && MIGRATION_DIR.is_match(&f.filename)
        })
        .collect();
    if migrations.is_empty() {
        return Vec::new();
    }
    vec![finding(
        "migration-added",
        format!(
            "{} new database migration{}",
            migrations.len(),
            if migrations.len() == 1 { "" } else { "s" }
        ),
        "Migration files change production schema. Confirm they're safe under concurrent writes (NOT NULL columns without defaults, dropped/renamed columns, etc. deserve extra scrutiny).",
        Some(migrations[0].filename.clone()),
        Severity::Medium,
        0.85,
    )]
}

fn check_swallowed_catch(files: &[FileDiff]) -> Vec<PolicyFinding> {
    files
        .iter()
        .filter_map(|f| {
            let patch = f.patch.as_deref()?;
            SWALLOWED_CATCH.is_match(patch).then(|| {
                finding(
                    "swallowed-catch",
                    "Empty catch block introduced",
                    format!("{} adds a catch block that swallows the error without logging or rethrowing.", f.filename),
                    Some(f.filename.clone()),
                    Severity::Medium,
                    0.85,
                )
            })
        })
        .collect()
}

fn check_hardcoded_localhost(files: &[FileDiff]) -> Vec<PolicyFinding> {
    let mut out = Vec::new();
    for f in files {
        let Some(patch) = f.patch.as_deref() else {
            continue;
        };
        if TEST_PATH.is_match(&f.filename) || DOC_OR_TEMPLATE_PATH.is_match(&f.filename) {
            continue;
        }
        let added = added_lines(patch);
        if LOCALHOST_URL.is_match(&added) {
            out.push(finding(
                "hardcoded-localhost",
                "Hardcoded localhost URL",
                format!("{} adds a hardcoded localhost/127.0.0.1 URL. Move it to a config/env var before shipping.", f.filename),
                Some(f.filename.clone()),
                Severity::Medium,
                0.8,
            ));
        }
    }
    out
}

/// Run every deterministic rule over a set of file diffs. Pure, synchronous,
/// no I/O — safe to call on every file-change event without rate limits.
pub fn run_policy_checks(files: &[FileDiff]) -> Vec<PolicyFinding> {
    let mut findings = Vec::new();
    findings.extend(check_missing_tests(files));
    findings.extend(check_debug_code(files));
    findings.extend(check_dependency_change(files));
    findings.extend(check_secrets(files));
    findings.extend(check_removed_tests(files));
    findings.extend(check_large_file_change(files));
    findings.extend(check_direct_db_access_in_handler(files));
    findings.extend(check_migration_added(files));
    findings.extend(check_swallowed_catch(files));
    findings.extend(check_hardcoded_localhost(files));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_secret_in_added_line() {
        let files = vec![FileDiff {
            filename: "src/config.ts".into(),
            status: "modified".into(),
            additions: 1,
            deletions: 0,
            patch: Some("+const key = \"AKIAABCDEFGHIJKLMNOP\";".into()),
        }];
        let findings = run_policy_checks(&files);
        assert!(findings.iter().any(|f| f.rule_key == "secret-in-diff"));
    }

    #[test]
    fn ignores_fixture_paths_for_secrets() {
        let files = vec![FileDiff {
            filename: "src/__fixtures__/keys.example.ts".into(),
            status: "added".into(),
            additions: 1,
            deletions: 0,
            patch: Some("+const key = \"AKIAABCDEFGHIJKLMNOP\";".into()),
        }];
        let findings = run_policy_checks(&files);
        assert!(!findings.iter().any(|f| f.rule_key == "secret-in-diff"));
    }

    #[test]
    fn flags_missing_tests_on_payment_path_without_test_touch() {
        let files = vec![FileDiff {
            filename: "src/billing/charge.ts".into(),
            status: "modified".into(),
            additions: 20,
            deletions: 3,
            patch: Some("+doCharge();".into()),
        }];
        let findings = run_policy_checks(&files);
        assert!(findings
            .iter()
            .any(|f| f.rule_key == "missing-tests-for-payments-paths"));
    }
}
