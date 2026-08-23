//! `trc ratify <pr>` — ratify a GitHub pull request against the deterministic
//! policy engine, straight from a git checkout. No daemon, no `trc init`, no
//! API key. Resolves the repo from the current directory's `origin` remote and
//! a read-only token (env, `gh` CLI, or `~/.trace/github.json`), fetches the
//! PR's changed files, and prints findings + a pass/review/block verdict.
//!
//! Shares `trace_core::ratify::summarize` with the daemon's `/github/ratify`
//! endpoint, so the terminal and the dashboard always agree.

use anyhow::{Context, Result};
use trace_core::{git, github, ratify_summarize, run_policy_checks, RatifyVerdict};

use crate::colors;

pub struct RatifyOptions {
    /// Pull-request number to ratify.
    pub pr: i64,
    /// Exit non-zero when the verdict is `block` (a high-severity finding).
    pub fail_on_risky: bool,
}

pub fn run(opts: RatifyOptions) -> Result<()> {
    let root = std::env::current_dir().context("reading current directory")?;
    if !git::is_git_repo(&root) {
        anyhow::bail!("not a git repository: {}", root.display());
    }

    let remote = git::remote_url(&root)
        .context("this repo has no git remote — Ratify needs a GitHub origin")?;
    let repo = github::parse_remote(&remote)
        .with_context(|| format!("origin remote is not a GitHub repo: {remote}"))?;
    let (token, source) = github::resolve_token();

    println!(
        "{} {}/{} {}",
        colors::bold("Trace ratify"),
        repo.owner,
        repo.repo,
        colors::dim(&format!("PR #{} · token: {}", opts.pr, source.as_str()))
    );

    let files = github::list_pr_files(&repo, token.as_deref(), opts.pr).with_context(|| {
        format!(
            "fetching files for PR #{} (does it exist? is the repo private?)",
            opts.pr
        )
    })?;
    let findings = run_policy_checks(&files);
    let summary = ratify_summarize(&findings);

    if findings.is_empty() {
        println!(
            "\nPolicy engine: no findings across {} file(s).",
            files.len()
        );
    } else {
        println!(
            "\nPolicy engine: {} finding(s) across {} file(s)",
            findings.len(),
            files.len()
        );
        for f in &findings {
            let tag = match f.severity {
                trace_core::Severity::High => colors::red("[high]"),
                trace_core::Severity::Medium => colors::yellow("[medium]"),
                trace_core::Severity::Low => colors::dim("[low]"),
            };
            println!(
                "  {tag} {} — {}",
                f.title,
                f.file_path.as_deref().unwrap_or("(no path)")
            );
        }
    }

    let verdict_str = summary.verdict.as_str();
    let painted = match summary.verdict {
        RatifyVerdict::Block => colors::red(verdict_str),
        RatifyVerdict::Review => colors::yellow(verdict_str),
        RatifyVerdict::Pass => colors::green(verdict_str),
    };
    println!(
        "\nVerdict: {}  ({} high · {} medium · {} low)",
        painted, summary.counts.high, summary.counts.medium, summary.counts.low
    );

    if opts.fail_on_risky && summary.verdict.is_block() {
        std::process::exit(1);
    }
    Ok(())
}
