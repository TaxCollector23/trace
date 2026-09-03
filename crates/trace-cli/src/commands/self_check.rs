//! `trc self-check` — runs Trace's own policy-engine benchmark: labeled
//! fixtures through the real `run_policy_checks`, scored for precision and
//! recall. A quick way to sanity-check a build, and the same computation
//! the dashboard's Benchmarks card shows.

use anyhow::Result;
use trace_core::{run_policy_eval, run_redteam_eval};

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
        let status = if r.passed {
            colors::green("PASS")
        } else {
            colors::red("FAIL")
        };
        let expected = r.expected_rule.as_deref().unwrap_or("(nothing)");
        println!(
            "  [{status}] {} — expected: {expected}, fired: {:?}",
            r.name, r.fired_rules
        );
    }

    // --- Red-team detection benchmark (guard / secrets / prompt) ---
    let rt = run_redteam_eval();
    println!("\n{}", colors::bold("Trace red-team detection benchmark"));
    println!(
        "  {}/{} threats caught  ·  {} false positive(s)  ·  recall {:.0}%\n",
        rt.total_caught(),
        rt.total_threats(),
        rt.total_false_positives(),
        rt.overall_recall() * 100.0
    );
    for e in &rt.engines {
        let clean = e.missed == 0 && e.downgraded == 0 && e.false_positives == 0;
        let status = if clean {
            colors::green("PASS")
        } else {
            colors::red("FAIL")
        };
        println!(
            "  [{status}] {:<16} {}/{} caught, {} missed, {} downgraded, {} false+ ({} benign)",
            e.name, e.caught, e.threats, e.missed, e.downgraded, e.false_positives, e.benign
        );
    }
    println!(
        "  {} rule pack {} · {} injection phrases · {} command rules · {} secret patterns",
        colors::dim("pack:"),
        rt.pack_version,
        rt.injection_phrases,
        rt.command_rules,
        rt.secret_patterns
    );

    // A failed fixture is an intentional, expected gate outcome (a policy
    // rule regressed) — not a crash — so print a clear summary and exit
    // directly rather than going through the generic error path (which
    // would append a "Re-run with TRACE_DEBUG=1" footer meant for real
    // internal failures).
    let policy_failed = report.passed < report.total;
    let redteam_failed = !rt.passed;
    if should_fail(policy_failed, redteam_failed) {
        if policy_failed {
            println!(
                "\n{} {} of {} policy fixtures failed — a policy rule regressed.",
                colors::red("self-check failed:"),
                report.total - report.passed,
                report.total
            );
        }
        if redteam_failed {
            println!(
                "\n{} a red-team detection engine regressed — see rows above.",
                colors::red("self-check failed:")
            );
        }
        std::process::exit(1);
    }
    println!(
        "\n{}",
        colors::green("All fixtures and red-team threats passed.")
    );
    Ok(())
}

/// The CI gate: fail when either the policy fixtures or the red-team
/// benchmark regressed.
fn should_fail(policy_failed: bool, redteam_failed: bool) -> bool {
    policy_failed || redteam_failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_fails_when_either_benchmark_regresses() {
        assert!(!should_fail(false, false));
        assert!(should_fail(true, false));
        assert!(should_fail(false, true));
        assert!(should_fail(true, true));
    }
}
