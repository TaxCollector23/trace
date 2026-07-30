//! Doctrine mining.
//!
//! Ported from Ratify's `miner.ts`, adapted to Trace's simpler token-based
//! GitHub access (`github.rs::resolve_token`) instead of a full GitHub App —
//! mining is a user-triggered, read-only, local operation, so it doesn't
//! need installation tokens or webhook infrastructure, just a token that can
//! read the repo's PR history. Uses one of the judge panel's own configured
//! provider slots to do the extraction (no separate key needed).

use serde::{Deserialize, Serialize};

use crate::github::{self, RepoRef};
use crate::judge::{call_provider_raw, JudgeSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStrength {
    HardRule,
    SoftNorm,
    LikelyPreference,
}

impl RuleStrength {
    fn parse(s: &str) -> RuleStrength {
        match s.trim().to_lowercase().as_str() {
            "hard-rule" => RuleStrength::HardRule,
            "likely-preference" => RuleStrength::LikelyPreference,
            _ => RuleStrength::SoftNorm,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RuleStrength::HardRule => "hard-rule",
            RuleStrength::SoftNorm => "soft-norm",
            RuleStrength::LikelyPreference => "likely-preference",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinedRule {
    pub rule_key: String,
    pub rule_text: String,
    pub category: String,
    pub strength: RuleStrength,
    pub confidence: f64,
    pub supporting_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResult {
    pub rules: Vec<MinedRule>,
    pub prs_analyzed: usize,
}

#[derive(Deserialize)]
struct RawMinedRule {
    #[serde(rename = "ruleKey")]
    rule_key: String,
    #[serde(rename = "ruleText")]
    rule_text: String,
    category: String,
    strength: String,
    confidence: f64,
    #[serde(rename = "supportingEvidence", default)]
    supporting_evidence: Vec<String>,
}

#[derive(Deserialize)]
struct RawMiningResponse {
    rules: Vec<RawMinedRule>,
}

/// Mine doctrine from a repo's recent merged-PR review history. Returns an
/// empty rule list (not an error) when there's no judge provider configured
/// to do the extraction, or when the repo has no merged PRs yet — mining is
/// a "nice to have" enrichment, not something that should ever block the
/// rest of Trace.
pub fn mine_doctrine(r: &RepoRef, settings: &JudgeSettings, pr_sample_size: usize) -> anyhow::Result<MiningResult> {
    let (token, _source) = github::resolve_token();
    let token = token.as_deref();

    let prs = github::list_recent_merged_pulls(r, token, pr_sample_size)?;
    if prs.is_empty() {
        return Ok(MiningResult { rules: Vec::new(), prs_analyzed: 0 });
    }

    let Some(slot) = settings.slots.iter().find(|s| s.api_key.is_some()) else {
        // No usable provider — still report how much history exists so the
        // dashboard can say "12 merged PRs found, configure a judge
        // provider to mine doctrine from them" rather than just "0 rules".
        return Ok(MiningResult { rules: Vec::new(), prs_analyzed: prs.len() });
    };

    // Aggregate review + issue comments across the sampled PRs into one
    // corpus, capped so the prompt stays a sane size regardless of how
    // chatty the repo's review culture is.
    let mut corpus = String::new();
    for pr in &prs {
        let review = github::list_pr_review_comments(r, token, pr.number).unwrap_or_default();
        let issue = github::list_pr_issue_comments(r, token, pr.number).unwrap_or_default();
        if review.is_empty() && issue.is_empty() {
            continue;
        }
        corpus.push_str(&format!("## PR #{}: {}\n", pr.number, pr.title));
        for c in review.iter().take(6) {
            corpus.push_str(&format!("- [review] {}: {}\n", c.author, truncate(&c.body, 300)));
        }
        for c in issue.iter().take(4) {
            corpus.push_str(&format!("- [issue] {}: {}\n", c.author, truncate(&c.body, 300)));
        }
        corpus.push('\n');
        if corpus.len() > 22_000 {
            corpus.truncate(22_000);
            break;
        }
    }

    if corpus.trim().is_empty() {
        return Ok(MiningResult { rules: Vec::new(), prs_analyzed: prs.len() });
    }

    let prompt = format!(
        "You are analyzing merged pull request review comments from a single repository to extract that repository's *engineering doctrine* — the recurring rules and norms this team actually enforces during code review. Focus on patterns that appear more than once or are asserted with confidence, not one-off preferences.\n\n\
Repository: {}/{}\n\n\
Review comment corpus (most recent {} merged PRs):\n{}\n\n\
Respond with ONLY a JSON object of this exact shape, no prose outside the JSON:\n\
{{\n  \"rules\": [\n    {{\n      \"ruleKey\": \"kebab-case-identifier\",\n      \"ruleText\": \"One-sentence rule as it would be written in a style guide, imperative voice.\",\n      \"category\": \"one of: testing | architecture | documentation | dependencies | naming | security | performance | other\",\n      \"strength\": \"hard-rule | soft-norm | likely-preference\",\n      \"confidence\": 0.0 to 1.0,\n      \"supportingEvidence\": [\"PR #XXX: brief quote\", \"PR #YYY: brief quote\"]\n    }}\n  ]\n}}",
        r.owner, r.repo, prs.len(), corpus
    );

    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(30)).build();
    let content = call_provider_raw(slot, &prompt, &agent)?;
    let cleaned = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let parsed: RawMiningResponse = serde_json::from_str(cleaned)?;

    let rules = parsed
        .rules
        .into_iter()
        .map(|raw| MinedRule {
            rule_key: raw.rule_key,
            rule_text: raw.rule_text,
            category: raw.category,
            strength: RuleStrength::parse(&raw.strength),
            confidence: raw.confidence.clamp(0.0, 1.0),
            supporting_evidence: raw.supporting_evidence,
        })
        .collect();

    Ok(MiningResult { rules, prs_analyzed: prs.len() })
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
