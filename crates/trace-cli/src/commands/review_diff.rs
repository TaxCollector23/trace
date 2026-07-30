//! `trace review-diff` — review a diff range with the policy engine (and,
//! optionally, the judge panel) without needing a registered Trace project
//! or a running daemon. Built for CI: point it at a git checkout, get a
//! report and an exit code. This is the same `policy.rs`/`judge.rs` engine
//! the live daemon uses — CI and the local agent-monitoring path share one
//! implementation instead of drifting apart.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use trace_core::{git, judge, policy, Decision, GlobalConfig};

use crate::colors;

pub struct ReviewDiffOptions {
    /// A git range like `origin/main...HEAD`. Auto-detected from
    /// `GITHUB_BASE_REF` (set by GitHub Actions on `pull_request` events)
    /// when not given, falling back to `HEAD~1...HEAD` for a plain push.
    pub range: Option<String>,
    /// Run the 3-LLM judge panel in addition to the deterministic policy
    /// engine. Needs judge provider keys configured (env vars or
    /// `~/.trace/global.toml`) — degrades to policy-only with a note if none
    /// are set, rather than failing the whole command.
    pub judge: bool,
    /// Exit non-zero when the review finds something worth blocking on: any
    /// high-severity policy finding, or a judge consensus of
    /// `require_approval`/`block`.
    pub fail_on_risky: bool,
    /// Write the full structured result here in addition to stdout.
    pub json_out: Option<PathBuf>,
}

#[derive(Serialize)]
struct ReviewOutput {
    schema: &'static str,
    range: String,
    files_changed: usize,
    policy_findings: Vec<policy::PolicyFinding>,
    high_severity_count: usize,
    judge_verdict: Option<judge::JudgeVerdict>,
    should_fail: bool,
}

pub fn run(opts: ReviewDiffOptions) -> Result<()> {
    let root = std::env::current_dir().context("reading current directory")?;
    if !git::is_git_repo(&root) {
        anyhow::bail!("not a git repository: {}", root.display());
    }

    let range = opts.range.unwrap_or_else(default_range);
    println!("{} {}", colors::bold("Trace review-diff"), colors::dim(&range));

    let entries = git::diff_range(&root, &range)
        .with_context(|| format!("computing diff for range `{range}` (does it exist? try `git fetch` first)"))?;
    let patches = git::patches_by_file_range(&root, &range).unwrap_or_default();

    let diffs: Vec<policy::FileDiff> = entries
        .iter()
        .map(|e| policy::FileDiff {
            filename: e.path.clone(),
            status: e.change_type.as_diff_status().to_string(),
            additions: 0,
            deletions: 0,
            patch: patches.get(&e.path).cloned().or_else(|| e.diff_summary.clone()),
        })
        .collect();

    let findings = policy::run_policy_checks(&diffs);
    let high_severity_count = findings.iter().filter(|f| f.severity == policy::Severity::High).count();

    print_policy_report(&findings);

    let mut judge_verdict = None;
    if opts.judge {
        let cfg = GlobalConfig::load().unwrap_or_default();
        if cfg.judge.mode == judge::JudgeMode::Disabled || cfg.judge.slots.iter().all(|s| s.api_key.is_none()) {
            println!(
                "\n{}",
                colors::dim("Judge requested but no provider keys are configured (TRACE_<PROVIDER>_API_KEY) — skipping the panel, policy findings above still apply.")
            );
        } else {
            println!("\n{}", colors::dim("Running the 3-LLM judge panel..."));
            let ctx = judge::JudgeContext {
                subject: format!("CI review of {range}"),
                agent_name: None,
                user_prompt: None,
                command: None,
                files: diffs.clone(),
                policy_findings: findings.clone(),
                doctrine_rules: Vec::new(),
            };
            let verdict = judge::run_judge(&cfg.judge, &ctx);
            print_judge_report(&verdict);
            judge_verdict = Some(verdict);
        }
    }

    let should_fail = opts.fail_on_risky
        && (high_severity_count > 0
            || judge_verdict
                .as_ref()
                .map(|v| matches!(v.consensus, Decision::RequireApproval | Decision::Block))
                .unwrap_or(false));

    let output = ReviewOutput {
        schema: "trace.review/v1",
        range,
        files_changed: entries.len(),
        policy_findings: findings,
        high_severity_count,
        judge_verdict,
        should_fail,
    };

    if let Some(path) = &opts.json_out {
        std::fs::write(path, serde_json::to_string_pretty(&output)?)
            .with_context(|| format!("writing {}", path.display()))?;
        println!("\nWrote {}", path.display());
    }

    if should_fail {
        println!("\n{}", colors::red("Trace review-diff: FAIL (risky change detected)"));
        std::process::exit(1);
    }
    println!("\n{}", colors::green("Trace review-diff: OK"));
    Ok(())
}

fn default_range() -> String {
    if let Ok(base) = std::env::var("GITHUB_BASE_REF") {
        if !base.trim().is_empty() {
            return format!("origin/{}...HEAD", base.trim());
        }
    }
    "HEAD~1...HEAD".to_string()
}

fn print_policy_report(findings: &[policy::PolicyFinding]) {
    if findings.is_empty() {
        println!("Policy engine: no findings.");
        return;
    }
    println!("Policy engine: {} finding(s)", findings.len());
    for f in findings {
        let tag = match f.severity {
            policy::Severity::High => colors::red("[high]"),
            policy::Severity::Medium => colors::yellow("[medium]"),
            policy::Severity::Low => colors::dim("[low]"),
        };
        println!("  {tag} {} — {}", f.title, f.file_path.as_deref().unwrap_or("(no path)"));
    }
}

fn print_judge_report(v: &judge::JudgeVerdict) {
    let consensus = match v.consensus {
        Decision::Allow => colors::green("allow"),
        Decision::Warn => colors::yellow("warn"),
        Decision::RequireApproval => colors::yellow("require_approval"),
        Decision::Block => colors::red("block"),
    };
    println!("Judge panel: {consensus} ({}% agreement, {}% confidence)", (v.agreement * 100.0) as i32, (v.confidence * 100.0) as i32);
    println!("  {}", v.summary);
    for vote in &v.votes {
        match &vote.error {
            Some(e) => println!("  - {}: failed ({e})", vote.provider),
            None => println!("  - {}: {} — {}", vote.provider, vote.decision.as_str(), vote.reasoning),
        }
    }
}
