//! Prompt-quality analysis.
//!
//! v1 is a fast, deterministic heuristic pass — no LLM call, so it can run
//! synchronously on every prompt without adding latency or API cost. It
//! feeds `prompt_events` for the dashboard's coaching view (`db.rs`).
//!
//! This is intentionally not the final word on prompt quality: the plan is
//! for the dashboard to periodically batch a run's prompts through the same
//! 3-LLM judge panel (`judge.rs`) for deeper, semantic coaching feedback
//! ("this conflicts with what you asked two prompts ago") — that pass is
//! amortized (batched, off the hot path) rather than per-keystroke, unlike
//! the checks here.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptPattern {
    /// Fewer than 6 words — almost never enough context for a nontrivial task.
    TooShort,
    /// Leans on vague placeholders ("fix it", "handle this", "make it better")
    /// without saying what "it" is or what "better" means.
    Vague,
    /// Presents multiple options without picking one, or hedges heavily
    /// ("maybe", "I guess", "not sure, but").
    OpenEnded,
    /// Contains what look like contradictory constraints in the same prompt.
    Conflicting,
    /// Longer prompt with no stated acceptance criteria — no "should",
    /// "must", "when X then Y", or test/verification mention.
    NoAcceptanceCriteria,
    /// References concrete files, functions, or identifiers — a strong
    /// positive signal, surfaced so the dashboard can praise good habits
    /// too, not just flag bad ones.
    WellScoped,
}

impl PromptPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptPattern::TooShort => "too_short",
            PromptPattern::Vague => "vague",
            PromptPattern::OpenEnded => "open_ended",
            PromptPattern::Conflicting => "conflicting",
            PromptPattern::NoAcceptanceCriteria => "no_acceptance_criteria",
            PromptPattern::WellScoped => "well_scoped",
        }
    }

    /// One-line coaching tip shown in the dashboard next to a flagged prompt.
    pub fn advice(&self) -> &'static str {
        match self {
            PromptPattern::TooShort => "Give the agent more to work with: what file or feature, what the current behavior is, and what you want instead.",
            PromptPattern::Vague => "Replace placeholders like \"fix it\" or \"handle this\" with the specific symptom and the specific file/function.",
            PromptPattern::OpenEnded => "Pick one approach yourself, or explicitly ask the agent to propose options before implementing — don't leave it to guess which you meant.",
            PromptPattern::Conflicting => "Two parts of this prompt appear to pull in different directions — reread it for constraints that contradict each other.",
            PromptPattern::NoAcceptanceCriteria => "State how you'll know it worked: a test that should pass, a behavior to verify, or an example input/output.",
            PromptPattern::WellScoped => "Good — this prompt names concrete files/functions, which sharply narrows what the agent has to guess.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAnalysis {
    pub word_count: i64,
    /// 0-100. Higher is clearer. Starts at a neutral baseline and moves with
    /// detected patterns; not a precise measurement, a coaching signal.
    pub clarity_score: f64,
    pub patterns: Vec<PromptPattern>,
}

static VAGUE_PHRASES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(fix (it|this)|handle (it|this)|make it (better|work|nicer)|clean (it|this) up|do the (thing|needful)|you know what to do|figure it out)\b").unwrap()
});
static HEDGING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(maybe|i guess|not sure|i think|possibly|whatever works|either way)\b").unwrap());
static ACCEPTANCE_SIGNAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(should|must|when .* then|so that|make sure|test|verify|expect(ed)?|acceptance)\b").unwrap()
});
static SCOPE_SIGNAL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"`[^`]+`|\b[A-Za-z_][A-Za-z0-9_]*\.(rs|ts|tsx|js|jsx|py|go|rb)\b|\b[a-z]+(?:[A-Z][a-z]*)+\b").unwrap());
static CONFLICT_PAIRS: Lazy<Vec<(Regex, Regex)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"(?i)\bdon'?t (touch|change|modify)\b").unwrap(), Regex::new(r"(?i)\b(refactor|rewrite|update)\b").unwrap()),
        (Regex::new(r"(?i)\bas (fast|quick(ly)?) as possible\b").unwrap(), Regex::new(r"(?i)\b(thoroughly|comprehensive(ly)?|exhaustive(ly)?)\b").unwrap()),
        (Regex::new(r"(?i)\bkeep it simple\b").unwrap(), Regex::new(r"(?i)\b(also add|and also|plus add|extra feature)\b").unwrap()),
    ]
});

/// Analyze a single prompt. Pure and synchronous — safe to call inline when
/// a run starts.
pub fn analyze_prompt(text: &str) -> PromptAnalysis {
    let word_count = text.split_whitespace().count() as i64;
    let mut patterns = Vec::new();
    let mut score: f64 = 75.0; // neutral baseline

    if word_count < 6 {
        patterns.push(PromptPattern::TooShort);
        score -= 30.0;
    }

    if VAGUE_PHRASES.is_match(text) {
        patterns.push(PromptPattern::Vague);
        score -= 20.0;
    }

    if HEDGING.is_match(text) {
        patterns.push(PromptPattern::OpenEnded);
        score -= 10.0;
    }

    if CONFLICT_PAIRS.iter().any(|(a, b)| a.is_match(text) && b.is_match(text)) {
        patterns.push(PromptPattern::Conflicting);
        score -= 25.0;
    }

    if word_count > 25 && !ACCEPTANCE_SIGNAL.is_match(text) {
        patterns.push(PromptPattern::NoAcceptanceCriteria);
        score -= 10.0;
    }

    if SCOPE_SIGNAL.is_match(text) {
        patterns.push(PromptPattern::WellScoped);
        score += 15.0;
    }

    PromptAnalysis {
        word_count,
        clarity_score: score.clamp(0.0, 100.0),
        patterns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_too_short() {
        let a = analyze_prompt("fix the bug");
        assert!(a.patterns.contains(&PromptPattern::TooShort));
    }

    #[test]
    fn flags_vague_placeholder() {
        let a = analyze_prompt("just fix it, you know what to do with the login page");
        assert!(a.patterns.contains(&PromptPattern::Vague));
    }

    #[test]
    fn rewards_well_scoped_prompt() {
        let a = analyze_prompt(
            "In `src/auth/session.ts`, the refreshSession function should reject expired tokens instead of silently renewing them. Add a test verifying this.",
        );
        assert!(a.patterns.contains(&PromptPattern::WellScoped));
        assert!(a.clarity_score > 75.0);
    }

    #[test]
    fn detects_conflicting_constraints() {
        let a = analyze_prompt("Don't touch the payments module, but please refactor the payments module to use the new client.");
        assert!(a.patterns.contains(&PromptPattern::Conflicting));
    }
}
