//! Adversarial red-team benchmark for Trace's detection engines.
//!
//! Runs the shared `trace_core::run_redteam_eval` corpus through the real
//! guard / secret / prompt detectors and prints a plain-text report. This is
//! the same computation behind `trc self-check` and the dashboard.
//!
//! Run: `cargo run -p trace-core --example redteam_bench`

use trace_core::run_redteam_eval;

fn main() {
    let report = run_redteam_eval();

    println!("\n================ Trace Red-Team Benchmark ================\n");
    println!(
        "{:<18} {:>8} {:>8} {:>10} {:>10} {:>8} {:>8}",
        "ENGINE", "THREATS", "CAUGHT", "PRECISION", "RECALL", "MISSED", "FALSE+"
    );
    for e in &report.engines {
        println!(
            "{:<18} {:>8} {:>8} {:>9.0}% {:>9.0}% {:>8} {:>8}",
            e.name,
            e.threats,
            e.caught,
            e.precision * 100.0,
            e.recall * 100.0,
            e.missed + e.downgraded,
            e.false_positives,
        );
    }

    println!("\n--- RULE PACK ---");
    println!("  active version   : {}", report.pack_version);
    println!("  injection phrases: {}", report.injection_phrases);
    println!("  command rules    : {}", report.command_rules);
    println!("  secret patterns  : {}", report.secret_patterns);

    println!("\n=========================================================");
    println!(
        "  Overall: {}/{} threats caught, {} false positive(s), precision {:.0}%, recall {:.0}%, F1 {:.0}%",
        report.total_caught(),
        report.total_threats(),
        report.total_false_positives(),
        report.overall_precision() * 100.0,
        report.overall_recall() * 100.0,
        report.overall_f1() * 100.0,
    );
    if report.passed {
        println!("RESULT: PASS — every planted threat caught, zero false positives.\n");
    } else {
        println!("RESULT: FAIL — a detection regressed. See rows above.\n");
        std::process::exit(1);
    }
}
