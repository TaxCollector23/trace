//! `trace check <file>` — run a file's contents through Trace's real detection
//! engines (command guard + secret scanner) and report what would be caught.
//!
//! Each non-comment line is classified by the same `guard::classify` the
//! runtime hook uses, and the whole file is swept for secrets. Exits non-zero
//! when anything at `require_approval` or `block` is found, so it doubles as a
//! CI gate for shell scripts and command lists.

use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use trace_core::guard::{classify, Decision};
use trace_core::secrets::scan_text;

use crate::colors;

fn rank(d: Decision) -> u8 {
    match d {
        Decision::Allow => 0,
        Decision::Warn => 1,
        Decision::RequireApproval => 2,
        Decision::Block => 3,
    }
}

fn paint(d: Decision, s: &str) -> String {
    match d {
        Decision::Block | Decision::RequireApproval => colors::red(s),
        Decision::Warn => colors::yellow(s),
        Decision::Allow => colors::green(s),
    }
}

/// Read the source: a real path, or stdin when the path is `-`.
fn read_source(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading from stdin")?;
        Ok(buf)
    } else {
        std::fs::read_to_string(Path::new(path)).with_context(|| format!("reading {path}"))
    }
}

/// A command-ish line worth classifying: skip blanks, comments, and shebangs.
fn is_command_line(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && !t.starts_with('#') && !t.starts_with("//")
}

pub fn run(path: &str) -> Result<()> {
    let source = read_source(path)?;
    let label = if path == "-" { "<stdin>" } else { path };

    println!("{}", colors::bold(&format!("Trace check — {label}")));
    println!(
        "{}\n",
        colors::dim("real guard + secret engines, no execution")
    );

    let mut worst = Decision::Allow;
    let mut flagged = 0usize;
    let mut scanned = 0usize;

    // --- Command guard, line by line ---
    for (i, raw) in source.lines().enumerate() {
        if !is_command_line(raw) {
            continue;
        }
        scanned += 1;
        let line = raw.trim();
        let res = classify(line);
        if res.decision == Decision::Allow {
            continue;
        }
        flagged += 1;
        if rank(res.decision) > rank(worst) {
            worst = res.decision;
        }
        let tag = paint(res.decision, res.decision.as_str());
        let shown: String = line.chars().take(70).collect();
        println!(
            "  {}  {}",
            colors::dim(&format!("L{:<3}", i + 1)),
            colors::bold(&shown)
        );
        println!("      {tag}  {}", colors::dim(&res.reason));
    }

    // --- Secret sweep over the whole file ---
    let secrets = scan_text(&source);
    if !secrets.is_empty() {
        if rank(Decision::Block) > rank(worst) {
            worst = Decision::Block;
        }
        println!(
            "\n  {}",
            colors::red(&format!("{} secret(s) detected:", secrets.len()))
        );
        for s in &secrets {
            println!(
                "      {}  {}",
                colors::red(&s.secret_type),
                colors::dim(&s.redacted_value)
            );
        }
    }

    // --- Summary ---
    println!();
    if flagged == 0 && secrets.is_empty() {
        println!(
            "{} {} command line(s) scanned — nothing risky.",
            colors::green("CLEAN"),
            scanned
        );
        return Ok(());
    }

    println!(
        "{} {} risky line(s) + {} secret(s) across {} scanned. Worst: {}",
        paint(worst, "FLAGGED"),
        flagged,
        secrets.len(),
        scanned,
        paint(worst, worst.as_str()),
    );

    // Exit non-zero when anything needs approval or a hard block, so CI can gate.
    if rank(worst) >= rank(Decision::RequireApproval) {
        anyhow::bail!(
            "check failed: found {} finding(s) at {} or higher",
            flagged + secrets.len(),
            Decision::RequireApproval.as_str()
        );
    }
    Ok(())
}
