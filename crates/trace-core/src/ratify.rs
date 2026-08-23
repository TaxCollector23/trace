//! Deterministic "ratification" of a set of changes (a PR's files, a diff): run
//! the policy engine and turn its findings into a single verdict.
//!
//! Pure, no I/O, no API key — the same rules that guard local edits. Shared by
//! the daemon's `/github/ratify` endpoint and the `trc ratify` CLI command so
//! the two can never drift.

use serde::Serialize;

use crate::policy::{PolicyFinding, Severity};

/// The overall verdict for a set of changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RatifyVerdict {
    /// Nothing flagged.
    Pass,
    /// Medium-severity findings only — worth a human look, not a hard stop.
    Review,
    /// At least one high-severity finding (e.g. a committed secret).
    Block,
}

impl RatifyVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            RatifyVerdict::Pass => "pass",
            RatifyVerdict::Review => "review",
            RatifyVerdict::Block => "block",
        }
    }

    /// True when the change trips a hard rule and should fail a gate.
    pub fn is_block(&self) -> bool {
        matches!(self, RatifyVerdict::Block)
    }
}

/// Per-severity counts of the findings behind a verdict.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct SeverityCounts {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// A verdict plus the counts that produced it. The findings themselves are
/// carried separately by callers (they already have them from the policy run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RatifySummary {
    pub verdict: RatifyVerdict,
    pub counts: SeverityCounts,
}

/// Summarize policy findings into a verdict: `block` if any high-severity
/// finding, `review` if only medium, else `pass`.
pub fn summarize(findings: &[PolicyFinding]) -> RatifySummary {
    let mut counts = SeverityCounts::default();
    for f in findings {
        match f.severity {
            Severity::High => counts.high += 1,
            Severity::Medium => counts.medium += 1,
            Severity::Low => counts.low += 1,
        }
    }
    let verdict = if counts.high > 0 {
        RatifyVerdict::Block
    } else if counts.medium > 0 {
        RatifyVerdict::Review
    } else {
        RatifyVerdict::Pass
    };
    RatifySummary { verdict, counts }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(sev: Severity) -> PolicyFinding {
        PolicyFinding {
            rule_key: "test".into(),
            title: "t".into(),
            description: "d".into(),
            file_path: None,
            severity: sev,
            confidence: 1.0,
            source: "policy-engine".into(),
        }
    }

    #[test]
    fn no_findings_is_pass() {
        let s = summarize(&[]);
        assert_eq!(s.verdict, RatifyVerdict::Pass);
        assert!(!s.verdict.is_block());
        assert_eq!(s.counts, SeverityCounts::default());
    }

    #[test]
    fn medium_only_is_review() {
        let s = summarize(&[finding(Severity::Medium), finding(Severity::Low)]);
        assert_eq!(s.verdict, RatifyVerdict::Review);
        assert_eq!(s.counts.medium, 1);
        assert_eq!(s.counts.low, 1);
        assert_eq!(s.counts.high, 0);
    }

    #[test]
    fn any_high_is_block() {
        let s = summarize(&[
            finding(Severity::Low),
            finding(Severity::High),
            finding(Severity::Medium),
        ]);
        assert_eq!(s.verdict, RatifyVerdict::Block);
        assert!(s.verdict.is_block());
        assert_eq!(s.counts.high, 1);
    }

    #[test]
    fn verdict_serializes_snake_case() {
        assert_eq!(
            serde_json::to_value(RatifyVerdict::Block).unwrap(),
            serde_json::json!("block")
        );
        assert_eq!(
            serde_json::to_value(RatifyVerdict::Review).unwrap(),
            serde_json::json!("review")
        );
        assert_eq!(
            serde_json::to_value(RatifyVerdict::Pass).unwrap(),
            serde_json::json!("pass")
        );
    }
}
