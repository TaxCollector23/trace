//! `trace self-check` — runs Trace's own policy-engine benchmark: labeled
//! fixtures through the real `run_policy_checks`, scored for precision and
//! recall. A quick way to sanity-check a build, and the same computation
//! the dashboard's Benchmarks card shows.

use anyhow::Result;
use trace_core::run_policy_eval;

use crate::colors;

pub fn run() -> Result<()> {
    let report = run_policy_eval();

    println!("{}", colors::bold("Trace policy engine self-check"));
    println!(
        "  {} / {} fixtures passed  ·  precision {:.0}%  ·  recall {:.0}%\n",
        report.passed,
        report.total,
        report.precision * 100.0,
        report.recall * 100.0
    );

    for r in &report.results {
        let status = if r.passed { colors::green("PASS") } else { colors::red("FAIL") };
        let expected = r.expected_rule.as_deref().unwrap_or("(nothing)");
        println!("  [{status}] {} — expected: {expected}, fired: {:?}", r.name, r.fired_rules);
    }

    if report.passed < report.total {
        anyhow::bail!(
            "{} of {} fixtures failed — a policy rule regressed",
            report.total - report.passed,
            report.total
        );
    }
    println!("\n{}", colors::green("All fixtures passed."));
    Ok(())
}
