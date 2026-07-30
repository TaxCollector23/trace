//! Prompt hardening, scoring, and linting.
//!
//! Hardening takes a user prompt and produces a stricter, agent-ready version:
//! the cleaned instruction plus a rules block that blocks fake work, fake
//! claims, and unnecessary output. All local and deterministic.

use serde::{Deserialize, Serialize};

use crate::prompt;

/// Hardening mode — controls which rule block is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HardenMode {
    Minimal,
    #[default]
    Balanced,
    Strict,
    Coding,
    Research,
    Marketing,
    Debate,
    School,
    Business,
    Investor,
    Design,
    Debugging,
    AppBuilder,
    ReleaseEngineer,
    Documentation,
    Security,
    NoBullshit,
}

impl HardenMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            HardenMode::Minimal => "minimal",
            HardenMode::Balanced => "balanced",
            HardenMode::Strict => "strict",
            HardenMode::Coding => "coding",
            HardenMode::Research => "research",
            HardenMode::Marketing => "marketing",
            HardenMode::Debate => "debate",
            HardenMode::School => "school",
            HardenMode::Business => "business",
            HardenMode::Investor => "investor",
            HardenMode::Design => "design",
            HardenMode::Debugging => "debugging",
            HardenMode::AppBuilder => "app-builder",
            HardenMode::ReleaseEngineer => "release-engineer",
            HardenMode::Documentation => "documentation",
            HardenMode::Security => "security",
            HardenMode::NoBullshit => "no-bullshit",
        }
    }

    pub fn parse(s: &str) -> HardenMode {
        match s.trim().to_lowercase().replace('_', "-").as_str() {
            "minimal" => HardenMode::Minimal,
            "strict" => HardenMode::Strict,
            "coding" => HardenMode::Coding,
            "research" => HardenMode::Research,
            "marketing" => HardenMode::Marketing,
            "debate" => HardenMode::Debate,
            "school" => HardenMode::School,
            "business" => HardenMode::Business,
            "investor" => HardenMode::Investor,
            "design" => HardenMode::Design,
            "debugging" => HardenMode::Debugging,
            "app-builder" => HardenMode::AppBuilder,
            "release-engineer" => HardenMode::ReleaseEngineer,
            "documentation" | "docs" => HardenMode::Documentation,
            "security" => HardenMode::Security,
            "no-bullshit" | "nobs" => HardenMode::NoBullshit,
            _ => HardenMode::Balanced,
        }
    }

    pub fn all() -> &'static [HardenMode] {
        use HardenMode::*;
        &[
            Minimal,
            Balanced,
            Strict,
            Coding,
            Research,
            Marketing,
            Debate,
            School,
            Business,
            Investor,
            Design,
            Debugging,
            AppBuilder,
            ReleaseEngineer,
            Documentation,
            Security,
            NoBullshit,
        ]
    }
}

/// Universal anti-hallucination rules attached to every non-minimal mode.
const UNIVERSAL: &[&str] = &[
    "Do not invent facts, files, APIs, or citations.",
    "If you are unsure or missing context, say so — do not guess.",
    "Do not claim work is complete unless it is actually complete.",
    "State assumptions explicitly.",
];

fn mode_rules(mode: HardenMode) -> Vec<&'static str> {
    use HardenMode::*;
    match mode {
        Minimal => vec![],
        Balanced => vec![
            "Be clear and direct.",
            "Ask before making large or destructive changes.",
        ],
        Strict => vec![
            "Follow the instructions exactly. Do nothing extra.",
            "No filler, no preamble, no summary unless asked.",
            "If a step cannot be done, stop and report why.",
        ],
        Coding | AppBuilder => vec![
            "Inspect the relevant files before editing.",
            "Do not invent files; do not delete files without asking.",
            "Make the smallest working change; do not change unrelated files.",
            "Preserve existing UI and behavior unless asked to change them.",
            "After changes, run the tests and report results honestly.",
            "Show only the changed code and the exact commands to run.",
            "No fake success, no fake screenshots, no fake deployment.",
            "Stop if required credentials or config are missing.",
        ],
        Debugging => vec![
            "Reproduce or locate the failure before proposing a fix.",
            "Explain the root cause, then the minimal fix.",
            "Give exact commands to verify the fix.",
            "Do not refactor unrelated code.",
        ],
        ReleaseEngineer => vec![
            "Verify version, changelog, tag, and build before claiming a release.",
            "List exact release commands; never claim a publish that did not run.",
            "Confirm artifact URLs and checksums.",
        ],
        Research => vec![
            "Cite concrete sources; mark anything uncertain.",
            "Separate established facts from speculation.",
            "Prefer primary sources; do not fabricate references.",
        ],
        Marketing => vec![
            "No hype, no buzzwords, no fake metrics.",
            "Be specific and credible; match the requested tone.",
        ],
        Debate => vec![
            "Steelman both sides before concluding.",
            "Flag weak or unsupported claims.",
        ],
        School => vec![
            "Explain reasoning step by step.",
            "Do not fabricate sources or quotes.",
        ],
        Business => vec![
            "Be concise and decision-oriented.",
            "State assumptions and risks explicitly.",
        ],
        Investor => vec![
            "Lead with the ask, traction, and risks.",
            "No inflated numbers; label projections as projections.",
        ],
        Design => vec![
            "Respect existing design system and constraints.",
            "Justify changes; avoid unnecessary redesigns.",
        ],
        Documentation => vec![
            "Document only what actually exists; mark planned items as planned.",
            "Include exact commands and accurate paths.",
        ],
        Security => vec![
            "Never output real secrets; redact sensitive values.",
            "Call out risky operations and least-privilege alternatives.",
            "Do not suggest disabling security controls without warning.",
        ],
        NoBullshit => vec![
            "Be direct. Fewest words that preserve correctness.",
            "No filler, no sugarcoating, no apologies unless there was a real failure.",
            "No fake completion claims. Say what changed, what failed, what remains.",
            "Bullets over paragraphs. Exact commands over vague advice.",
        ],
    }
}

/// Result of hardening a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardenResult {
    pub mode: String,
    pub original: String,
    pub hardened: String,
    pub score_before: PromptScore,
    pub score_after: PromptScore,
    pub lint: Vec<String>,
    pub rules_applied: Vec<String>,
}

/// Harden a prompt for the given mode, optionally adding project context and
/// custom rules. `coding` forces the coding rule block on top of the mode.
pub fn harden(
    input: &str,
    mode: HardenMode,
    project_context: Option<&str>,
    custom_rules: &[String],
) -> HardenResult {
    // 1. Clean the instruction (reuse the local compressor in concise mode).
    let cleaned = prompt::compress_with_mode(input, prompt::CompressionMode::Concise).compressed;
    let instruction = if cleaned.trim().is_empty() {
        input.trim().to_string()
    } else {
        cleaned
    };

    // 2. Assemble rules: universal (unless minimal) + mode + custom.
    let mut rules: Vec<String> = Vec::new();
    if mode != HardenMode::Minimal {
        rules.extend(UNIVERSAL.iter().map(|s| s.to_string()));
    }
    rules.extend(mode_rules(mode).iter().map(|s| s.to_string()));
    rules.extend(custom_rules.iter().cloned());

    // 3. Build the hardened prompt.
    let mut out = String::new();
    out.push_str("# Task\n");
    out.push_str(instruction.trim());
    out.push('\n');
    if let Some(ctx) = project_context {
        if !ctx.trim().is_empty() {
            out.push_str("\n# Project context\n");
            out.push_str(ctx.trim());
            out.push('\n');
        }
    }
    if !rules.is_empty() {
        out.push_str("\n# Rules\n");
        for r in &rules {
            out.push_str("- ");
            out.push_str(r);
            out.push('\n');
        }
    }

    HardenResult {
        mode: mode.as_str().to_string(),
        original: input.to_string(),
        hardened: out.trim_end().to_string(),
        score_before: score(input),
        score_after: score(&out),
        lint: lint(input),
        rules_applied: rules,
    }
}

/// A single scored metric (0–100).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: u8,
}

/// Overall prompt score plus per-metric breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptScore {
    pub overall: u8,
    pub metrics: Vec<Metric>,
}

const VAGUE: &[&str] = &[
    "make it better",
    "fix everything",
    "do all of it",
    "make it good",
    "just make it work",
    "make it nice",
    "improve it",
    "clean it up",
    "do the thing",
    "etc",
];

fn has_any(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}

fn clamp(v: i32) -> u8 {
    v.clamp(0, 100) as u8
}

/// Score a prompt across nine deterministic heuristics.
pub fn score(input: &str) -> PromptScore {
    let p = input.to_lowercase();
    let words = input.split_whitespace().count().max(1);
    let has_file = input.contains('/')
        || input.contains('.') && p.split_whitespace().any(|w| w.contains('.') && w.len() > 3);
    let has_cmd = has_any(
        &p,
        &[
            "npm ", "cargo ", "git ", "run ", "build", "test", "install", "deploy",
        ],
    );
    let has_output_fmt = has_any(
        &p,
        &[
            "format", "json", "bullet", "table", "markdown", "list", "steps", "diff",
        ],
    );
    let has_accept = has_any(
        &p,
        &[
            "test",
            "verify",
            "pass",
            "acceptance",
            "should",
            "must",
            "expect",
        ],
    );
    let vague = has_any(&p, VAGUE);
    let has_constraint = has_any(
        &p,
        &[
            "do not", "don't", "never", "only", "must", "without", "unless", "keep", "preserve",
        ],
    );

    let clarity = clamp(80 - if vague { 50 } else { 0 } + if has_constraint { 10 } else { 0 });
    let specificity = clamp(
        30 + if has_file { 35 } else { 0 }
            + if has_cmd { 25 } else { 0 }
            + if words > 12 { 10 } else { 0 },
    );
    let actionability = clamp(
        40 + if has_cmd { 30 } else { 0 } + if has_file { 15 } else { 0 }
            - if vague { 25 } else { 0 },
    );
    // Hallucination risk: report as resistance (higher = safer).
    let hallucination_risk =
        clamp(45 + if has_constraint { 35 } else { 0 } + if has_accept { 15 } else { 0 });
    let missing_context =
        clamp(35 + if has_file { 35 } else { 0 } + if words > 15 { 20 } else { 0 });
    let output_control =
        clamp(35 + if has_output_fmt { 45 } else { 0 } + if has_constraint { 10 } else { 0 });
    let testability = clamp(30 + if has_accept { 50 } else { 0 });
    let concision = clamp(if words <= 4 {
        50
    } else if words <= 60 {
        90
    } else if words <= 120 {
        70
    } else {
        45
    });
    let agent_readiness = clamp(
        ((clarity as i32)
            + (specificity as i32)
            + (actionability as i32)
            + (output_control as i32))
            / 4,
    );

    let metrics = vec![
        Metric {
            name: "clarity".into(),
            value: clarity,
        },
        Metric {
            name: "specificity".into(),
            value: specificity,
        },
        Metric {
            name: "actionability".into(),
            value: actionability,
        },
        Metric {
            name: "hallucination_resistance".into(),
            value: hallucination_risk,
        },
        Metric {
            name: "context_completeness".into(),
            value: missing_context,
        },
        Metric {
            name: "output_control".into(),
            value: output_control,
        },
        Metric {
            name: "testability".into(),
            value: testability,
        },
        Metric {
            name: "concision".into(),
            value: concision,
        },
        Metric {
            name: "agent_readiness".into(),
            value: agent_readiness,
        },
    ];
    let sum: i32 = metrics.iter().map(|m| m.value as i32).sum();
    let overall = clamp(sum / metrics.len() as i32);
    PromptScore { overall, metrics }
}

/// Lint a prompt for known bad patterns. Returns human-readable warnings.
pub fn lint(input: &str) -> Vec<String> {
    let p = input.to_lowercase();
    let mut out = Vec::new();
    for phrase in VAGUE {
        if p.contains(phrase) {
            out.push(format!(
                "Vague instruction: \"{phrase}\" — state a concrete, testable goal."
            ));
        }
    }
    if !has_any(
        &p,
        &[
            "test",
            "verify",
            "should",
            "must",
            "expect",
            "pass",
            "acceptance",
        ],
    ) {
        out.push("No success criteria — say how to verify the result.".to_string());
    }
    if !(input.contains('/') || p.split_whitespace().any(|w| w.contains('.') && w.len() > 3)) {
        out.push("No file or path mentioned — name the files involved.".to_string());
    }
    if !has_any(
        &p,
        &[
            "format",
            "json",
            "bullet",
            "table",
            "markdown",
            "list",
            "steps",
            "diff",
            "only show",
            "concise",
        ],
    ) {
        out.push("No output format specified — say what the answer should look like.".to_string());
    }
    // Conflicts: reuse the compressor's detector.
    out.extend(prompt::detect_conflicts(input));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vague_prompt_scores_low_and_lints() {
        let s = score("make it better");
        assert!(s.overall < 55, "expected low score, got {}", s.overall);
        let l = lint("make it better");
        assert!(l.iter().any(|w| w.contains("Vague")));
    }

    #[test]
    fn specific_prompt_scores_higher() {
        let vague = score("make it good").overall;
        let specific = score(
            "In src/auth/login.ts, fix the 401 bug; run npm test to verify. Do not change the UI.",
        )
        .overall;
        assert!(specific > vague);
    }

    #[test]
    fn coding_mode_adds_safety_rules() {
        let r = harden("fix my app", HardenMode::Coding, None, &[]);
        assert!(r.hardened.contains("Do not invent files"));
        assert!(r.hardened.contains("# Rules"));
        assert!(r.hardened.contains("smallest working change"));
    }

    #[test]
    fn minimal_mode_has_no_rule_block() {
        let r = harden("write a haiku about rust", HardenMode::Minimal, None, &[]);
        assert!(!r.hardened.contains("Do not invent files"));
    }

    #[test]
    fn custom_rules_and_context_included() {
        let r = harden(
            "fix the dashboard",
            HardenMode::Coding,
            Some("Framework: React + Vite. Tests: vitest."),
            &["Never touch the database schema".to_string()],
        );
        assert!(r.hardened.contains("Project context"));
        assert!(r.hardened.contains("React + Vite"));
        assert!(r.hardened.contains("Never touch the database schema"));
    }

    #[test]
    fn no_bullshit_mode() {
        let r = harden("explain this", HardenMode::NoBullshit, None, &[]);
        assert!(r.hardened.to_lowercase().contains("no filler"));
    }

    #[test]
    fn all_modes_parse_roundtrip() {
        for m in HardenMode::all() {
            assert_eq!(HardenMode::parse(m.as_str()), *m);
        }
    }
}
