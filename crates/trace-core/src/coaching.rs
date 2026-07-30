//! Prompt-coaching analytics.
//!
//! Reads the user's recent prompt events (see `db::recent_prompt_events`)
//! and produces personalized coaching feedback:
//!
//!   - Which patterns show up in *your* prompts most often?
//!   - How often does each pattern correlate with a run getting flagged
//!     by the deterministic policy engine or the LLM judge?
//!   - What specific concrete change would help — with a real example
//!     lifted from one of your own recent prompts?
//!
//! Deterministic aggregation only, no LLM call. Runs in milliseconds so the
//! dashboard can refresh it on every visit. The semantic "this prompt
//! conflicts with what you asked two prompts ago" pass belongs in the
//! amortized judge-panel batch job (`judge.rs`), not here.

use serde::{Deserialize, Serialize};

use crate::models::PromptEventRecord;
use crate::prompt_quality::PromptPattern;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStat {
    pub pattern: String,
    /// Times this pattern appeared in the sample window.
    pub occurrences: i64,
    /// Fraction of your prompts in the window with this pattern, [0,1].
    pub share: f64,
    /// Of the prompts with this pattern, the fraction whose run led to
    /// any policy or judge flag ("led_to_flag"). Higher = this pattern is
    /// costing *you* specifically, not a generic best-practice tip.
    pub flag_rate: f64,
    pub advice: String,
    /// A real, truncated example from the user's own prompt history.
    /// Nothing is more persuasive than "here's what you actually typed."
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachingReport {
    /// Total prompts in the sample window.
    pub sample_size: i64,
    /// Mean clarity score across the window, 0-100.
    pub avg_clarity: f64,
    /// Fraction of prompts that led to any flag, [0,1].
    pub overall_flag_rate: f64,
    /// Patterns ranked by expected coaching value: pattern-specific
    /// flag_rate weighted by how often it shows up in *your* prompts.
    /// Positive patterns (e.g. `well_scoped`) are included but sorted
    /// last so the coaching surface leads with what to *fix*.
    pub patterns: Vec<PatternStat>,
    /// One sentence summarising the top opportunity ("Your prompts are
    /// 2.3× more likely to get flagged when they're too short — here's
    /// one from yesterday you could rewrite: ..."). Human-readable, safe
    /// to render as-is.
    pub headline: String,
}

/// Build a coaching report from raw prompt events. Pure function — no I/O,
/// so easy to unit-test with a synthetic window.
pub fn build_report(events: &[PromptEventRecord]) -> CoachingReport {
    if events.is_empty() {
        return CoachingReport {
            sample_size: 0,
            avg_clarity: 0.0,
            overall_flag_rate: 0.0,
            patterns: Vec::new(),
            headline: "No prompts recorded yet — run any agent through `trace run` and coaching kicks in.".into(),
        };
    }
    let sample_size = events.len() as i64;
    let avg_clarity = events.iter().map(|e| e.clarity_score).sum::<f64>() / sample_size as f64;
    let overall_flag_rate = events.iter().filter(|e| e.led_to_flag).count() as f64 / sample_size as f64;

    // Bucket by pattern. Every pattern PromptPattern::as_str() we know about
    // gets a row so even zero-occurrence rows can render "you haven't fallen
    // into this trap" in a future UI.
    let patterns_of_interest = [
        PromptPattern::TooShort,
        PromptPattern::Vague,
        PromptPattern::OpenEnded,
        PromptPattern::Conflicting,
        PromptPattern::NoAcceptanceCriteria,
        PromptPattern::WellScoped,
    ];

    let mut rows: Vec<PatternStat> = patterns_of_interest
        .iter()
        .filter_map(|p| {
            let key = p.as_str();
            let matching: Vec<&PromptEventRecord> = events
                .iter()
                .filter(|e| e.patterns_json.contains(key))
                .collect();
            if matching.is_empty() {
                return None;
            }
            let flagged = matching.iter().filter(|e| e.led_to_flag).count();
            let example = pick_example(&matching);
            Some(PatternStat {
                pattern: key.into(),
                occurrences: matching.len() as i64,
                share: matching.len() as f64 / sample_size as f64,
                flag_rate: flagged as f64 / matching.len() as f64,
                advice: p.advice().into(),
                example,
            })
        })
        .collect();

    // Sort: positive patterns last, then descending by "coaching value" =
    // share * (flag_rate / overall_flag_rate). Falls back to raw share when
    // the overall rate is zero (nothing's been flagged yet — still worth
    // flagging what shows up most).
    rows.sort_by(|a, b| {
        let a_pos = a.pattern == "well_scoped";
        let b_pos = b.pattern == "well_scoped";
        if a_pos != b_pos {
            return a_pos.cmp(&b_pos); // positive last
        }
        let score = |r: &PatternStat| {
            if overall_flag_rate > 0.0 {
                r.share * (r.flag_rate / overall_flag_rate)
            } else {
                r.share
            }
        };
        score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal)
    });

    let headline = build_headline(&rows, overall_flag_rate);
    CoachingReport {
        sample_size,
        avg_clarity,
        overall_flag_rate,
        patterns: rows,
        headline,
    }
}

fn pick_example(matching: &[&PromptEventRecord]) -> Option<String> {
    // Prefer a flagged prompt (more instructive), fall back to any.
    let pick = matching.iter().find(|e| e.led_to_flag).or_else(|| matching.first())?;
    let text = pick.prompt_text.trim();
    if text.is_empty() {
        return None;
    }
    let truncated: String = text.chars().take(140).collect();
    Some(if text.chars().count() > 140 {
        format!("{truncated}…")
    } else {
        truncated
    })
}

fn build_headline(rows: &[PatternStat], overall_flag_rate: f64) -> String {
    // Skip the positive pattern; find the top actionable one.
    let top = rows.iter().find(|r| r.pattern != "well_scoped");
    let Some(top) = top else {
        return "Every prompt in your recent history was well-scoped — no coaching to add. Keep it up.".into();
    };
    if overall_flag_rate == 0.0 || top.flag_rate <= overall_flag_rate {
        return format!(
            "Most common pattern in your recent prompts: {} ({:.0}% of prompts). Not yet correlated with flagged runs — keep an eye on it as more data comes in.",
            humanize(&top.pattern),
            top.share * 100.0
        );
    }
    let lift = top.flag_rate / overall_flag_rate;
    let example_bit = top
        .example
        .as_deref()
        .map(|e| format!(" One yours: \"{e}\""))
        .unwrap_or_default();
    format!(
        "Your prompts are {:.1}× more likely to get flagged when they're {} ({:.0}% of the time vs. {:.0}% overall).{example_bit}",
        lift,
        humanize(&top.pattern),
        top.flag_rate * 100.0,
        overall_flag_rate * 100.0
    )
}

fn humanize(pattern: &str) -> String {
    match pattern {
        "too_short" => "too short",
        "vague" => "vague",
        "open_ended" => "open-ended",
        "conflicting" => "self-contradicting",
        "no_acceptance_criteria" => "missing acceptance criteria",
        "well_scoped" => "well-scoped",
        _ => pattern,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(text: &str, patterns: &[&str], clarity: f64, flagged: bool) -> PromptEventRecord {
        PromptEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            run_id: "r".into(),
            prompt_text: text.into(),
            word_count: text.split_whitespace().count() as i64,
            patterns_json: format!("[{}]", patterns.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join(",")),
            clarity_score: clarity,
            led_to_flag: flagged,
            created_at: "2026-07-30T00:00:00Z".into(),
        }
    }

    #[test]
    fn empty_events_produce_placeholder_headline() {
        let r = build_report(&[]);
        assert_eq!(r.sample_size, 0);
        assert!(r.headline.contains("No prompts"));
    }

    #[test]
    fn detects_lift_between_pattern_and_overall_flag_rate() {
        // 4 short prompts, all flagged. 6 well-scoped, none flagged. Overall
        // 40%; too_short 100% → 2.5× lift on the actionable pattern.
        let mut events = Vec::new();
        for _ in 0..4 {
            events.push(ev("fix it", &["too_short", "vague"], 20.0, true));
        }
        for _ in 0..6 {
            events.push(ev(
                "In apps/web/src/pages/Dashboard.tsx line 41, the click handler double-fires",
                &["well_scoped"],
                85.0,
                false,
            ));
        }
        let r = build_report(&events);
        assert_eq!(r.sample_size, 10);
        let top = &r.patterns[0];
        assert_eq!(top.pattern, "too_short");
        assert!(r.headline.contains("2.5"), "headline was: {}", r.headline);
        assert!(r.patterns.last().unwrap().pattern == "well_scoped");
    }
}
