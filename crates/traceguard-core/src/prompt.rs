//! TraceCompress — prompt compression and output discipline.
//!
//! Compression is **local and deterministic** by default — no model call, no
//! data leaves the machine. It removes filler and repetition while preserving
//! the things that change an agent's behaviour: commands, file names,
//! constraints, acceptance criteria, must-not-do rules, quoted strings, env
//! vars, URLs, and code blocks. It never rewrites intent into something vaguer
//! and never produces broken/unprofessional grammar.

use serde::{Deserialize, Serialize};

/// Compression aggressiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompressionMode {
    /// Light cleanup; preserve normal readability.
    Normal,
    /// Remove redundancy; keep complete sentences. (Default.)
    #[default]
    Concise,
    /// Maximum compression + strict response rules for coding agents.
    Bare,
}

impl CompressionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionMode::Normal => "normal",
            CompressionMode::Concise => "concise",
            CompressionMode::Bare => "bare",
        }
    }

    pub fn parse(s: &str) -> CompressionMode {
        match s.trim().to_lowercase().as_str() {
            "normal" => CompressionMode::Normal,
            "bare" => CompressionMode::Bare,
            _ => CompressionMode::Concise,
        }
    }
}

/// Result of compressing a prompt. Token counts are local estimates and must be
/// labelled as estimates in any UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompressionResult {
    pub mode: String,
    pub original: String,
    pub compressed: String,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    /// Percentage reduction in estimated tokens (0–100).
    pub reduction_pct: f64,
    /// Estimates only; not exact tokenizer output.
    pub estimated: bool,
    /// Constraints detected and kept verbatim (the preserved-constraints checklist).
    pub preserved_constraints: Vec<String>,
    /// Human-readable notes about what redundancy was removed.
    pub removed_redundancy: Vec<String>,
    /// Possible contradictory instructions. Non-empty means: ask the user.
    pub conflicts: Vec<String>,
    /// Response-control block to attach to the agent prompt (output discipline).
    pub response_rules: String,
}

/// Rough local token estimate. Without a provider tokenizer we approximate using
/// the common ~4-characters-per-token heuristic, with a word-count floor so
/// short prompts are not under-counted. Always treated as an estimate.
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    let by_chars = (chars as f64 / 4.0).ceil() as usize;
    by_chars.max(words)
}

/// Politeness/casual filler removed even in Normal mode.
const LIGHT_FILLER: &[&str] = &[
    "i would like you to",
    "i would like for you to",
    "i want you to please",
    "i was wondering if you could",
    "if you don't mind",
    "if you wouldn't mind",
    "thanks in advance",
    "thank you",
    "thanks",
    "please",
    "kindly",
    "bro",
    "dude",
    "man",
];

/// Additional filler removed in Concise / Bare modes.
const EXTRA_FILLER: &[&str] = &[
    "as you may know",
    "as you know",
    "needless to say",
    "for what it's worth",
    "at the end of the day",
    "to be honest",
    "i think that",
    "i think",
    "i believe",
    "i guess",
    "basically",
    "essentially",
    "literally",
    "honestly",
    "actually",
    "really",
    "very",
    "just",
    "simply",
    "kind of",
    "sort of",
    "like",
];

/// Words preceding "like" that mean it is NOT filler (e.g. "look like").
const LIKE_GUARD_PREV: &[&str] = &[
    "look", "looks", "feel", "feels", "sound", "sounds", "seem", "seems",
];

/// Markers that signal a line MUST be preserved verbatim. Includes the safety
/// keywords that must never be removed.
const CONSTRAINT_MARKERS: &[&str] = &[
    "must not",
    "must",
    "do not",
    "don't",
    "dont",
    "do not change",
    "do not add",
    "cannot",
    "can't",
    "never",
    "always",
    "only",
    "exactly",
    "without",
    "unless",
    "ensure",
    "require",
    "required",
    "acceptance",
    "constraint",
    "should not",
    "shall",
    "make sure",
    "keep",
    "preserve",
    "limit",
    "at most",
    "no more than",
    "deploy",
    "test",
];

fn looks_like_command(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('$')
        || t.starts_with("```")
        || t.contains("npm ")
        || t.contains("cargo ")
        || t.contains("git ")
        || t.contains("yarn ")
        || t.contains("pnpm ")
        || t.contains("pip ")
        || t.contains("python ")
        || t.contains("node ")
        || t.contains("./")
}

fn looks_like_path(token: &str) -> bool {
    let t = token.trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':' | ')' | '('));
    t.contains('/')
        || (t.contains('.')
            && t.rsplit('.').next().is_some_and(|ext| {
                (1..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric())
            }))
}

fn has_quoted_string(line: &str) -> bool {
    let q = line.matches('"').count();
    let b = line.matches('`').count();
    q >= 2 || b >= 2
}

fn has_env_or_url(line: &str) -> bool {
    line.contains("://")
        || line.split_whitespace().any(|w| {
            w.len() >= 3
                && w.chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                && w.contains('_')
        })
}

fn is_constraint_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    CONSTRAINT_MARKERS.iter().any(|m| lower.contains(m))
}

/// Whether a unit must be preserved verbatim (not filler-stripped).
fn must_preserve(line: &str) -> bool {
    looks_like_command(line)
        || is_constraint_line(line)
        || has_quoted_string(line)
        || has_env_or_url(line)
        || line.split_whitespace().any(looks_like_path)
}

/// Strip filler phrases, returning the cleaned text and the phrases removed.
fn strip_filler(text: &str, phrases: &[&str]) -> (String, Vec<String>) {
    let mut lower = text.to_lowercase();
    let mut out = text.to_string();
    let mut removed: Vec<String> = Vec::new();

    for phrase in phrases {
        while let Some(pos) = lower.find(phrase) {
            let before_ok = pos == 0 || !lower.as_bytes()[pos - 1].is_ascii_alphanumeric();
            let after = pos + phrase.len();
            let after_ok = after >= lower.len() || !lower.as_bytes()[after].is_ascii_alphanumeric();

            // Special case: keep "like" when it forms "look like", "feel like", etc.
            let like_ok = if *phrase == "like" {
                let prev_word = lower[..pos].split_whitespace().next_back().unwrap_or("");
                !LIKE_GUARD_PREV.contains(&prev_word)
            } else {
                true
            };

            if before_ok && after_ok && like_ok {
                out.replace_range(pos..after, " ");
                lower.replace_range(pos..after, " ");
                if !removed.contains(&phrase.to_string()) {
                    removed.push(phrase.to_string());
                }
            } else {
                // Skip past this occurrence to avoid an infinite loop.
                if after >= lower.len() {
                    break;
                }
                lower.replace_range(pos..pos + 1, "\u{0}");
            }
        }
        lower = lower.replace('\u{0}', " ");
    }
    (collapse_ws(&out), removed)
}

/// Split a line into sentences on `.`, `!`, `?` followed by whitespace.
fn split_sentences(line: &str) -> Vec<String> {
    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = line.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];
        current.push(c);
        if matches!(c, '.' | '!' | '?') {
            let next = chars.get(i + 1).copied();
            let prev = if i > 0 {
                chars.get(i - 1).copied()
            } else {
                None
            };
            let is_break = matches!(next, Some(n) if n.is_whitespace())
                && prev.map(|p| !p.is_ascii_digit()).unwrap_or(true);
            if is_break {
                sentences.push(current.trim().to_string());
                current.clear();
            }
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    sentences.retain(|s| !s.is_empty());
    sentences
}

/// Split a sentence into clauses on connectors so a run-on prompt can be
/// compressed clause-by-clause: filler clauses are dropped and constraint
/// clauses are preserved verbatim. Conservative — only common connectors.
fn split_clauses(sentence: &str) -> Vec<String> {
    let mut work = sentence.to_string();
    // Longest connectors first so they win over their substrings.
    for delim in [
        ", and also ",
        " and also ",
        ", and ",
        "; ",
        " but also ",
        " but ",
        " also ",
        " plus ",
        ", ",
        " and ",
    ] {
        work = work.replace(delim, "\u{1}");
    }
    work.split('\u{1}')
        .map(|s| strip_connector_words(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Trim leftover leading/trailing connector words from a split clause.
fn strip_connector_words(clause: &str) -> String {
    const CONNECTORS: &[&str] = &["and", "also", "but", "plus", "then", "so"];
    let mut words: Vec<&str> = clause.split_whitespace().collect();
    while words
        .first()
        .is_some_and(|w| CONNECTORS.contains(&w.to_lowercase().as_str()))
    {
        words.remove(0);
    }
    while words.last().is_some_and(|w| {
        CONNECTORS.contains(
            &w.trim_end_matches(['.', ',', '!', '?'])
                .to_lowercase()
                .as_str(),
        )
    }) {
        words.pop();
    }
    words.join(" ")
}

/// Normalized key for de-duplication: lowercase, trailing punctuation removed.
fn normalize_key(s: &str) -> String {
    s.to_lowercase()
        .trim_matches(|c: char| matches!(c, '.' | '!' | '?' | ',' | ' '))
        .to_string()
}

fn collapse_ws(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" ;", ";")
        .replace("  ", " ")
        .trim()
        .to_string()
}

/// Backwards-compatible entry point: compress in Concise mode.
pub fn compress(prompt: &str) -> CompressionResult {
    compress_with_mode(prompt, CompressionMode::Concise)
}

/// Compress a prompt deterministically in the given mode.
pub fn compress_with_mode(prompt: &str, mode: CompressionMode) -> CompressionResult {
    let original = prompt.to_string();
    let original_tokens = estimate_tokens(&original);

    let filler: Vec<&str> = match mode {
        CompressionMode::Normal => LIGHT_FILLER.to_vec(),
        _ => LIGHT_FILLER
            .iter()
            .chain(EXTRA_FILLER.iter())
            .copied()
            .collect(),
    };

    let mut seen_lines: Vec<String> = Vec::new();
    let mut seen_units: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut preserved: Vec<String> = Vec::new();
    let mut removed_filler: Vec<String> = Vec::new();
    let mut removed_dupes = 0usize;
    let mut in_code_block = false;

    for raw_line in original.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        // Preserve fenced code blocks verbatim.
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            seen_lines.push(line.to_string());
            continue;
        }
        if in_code_block {
            seen_lines.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            if mode != CompressionMode::Bare
                && seen_lines.last().map(|l| !l.is_empty()).unwrap_or(false)
            {
                seen_lines.push(String::new());
            }
            continue;
        }

        let is_list_item = trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed
                .split_once(['.', ')'])
                .map(|(h, _)| !h.is_empty() && h.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);

        // Normal mode keeps lines whole. Concise/Bare split prose into sentences
        // and then into clauses, so run-on prompts compress while constraint
        // clauses are preserved.
        let units: Vec<String> = if mode == CompressionMode::Normal || is_list_item {
            vec![line.to_string()]
        } else {
            split_sentences(line)
                .iter()
                .flat_map(|s| split_clauses(s))
                .collect()
        };

        // Clause-level output reads better joined by ", "; whole lines by " ".
        let joiner = if mode == CompressionMode::Normal || is_list_item {
            " "
        } else {
            ", "
        };
        let mut rebuilt: Vec<String> = Vec::new();
        for unit in units {
            let preserve = must_preserve(&unit);
            if preserve {
                collect_constraints(&unit, &mut preserved);
            }
            let processed = if preserve {
                unit.trim().to_string()
            } else {
                let (cleaned, removed) = strip_filler(&unit, &filler);
                for r in removed {
                    if !removed_filler.contains(&r) {
                        removed_filler.push(r);
                    }
                }
                cleaned
            };
            if processed.is_empty() {
                continue;
            }
            let key = normalize_key(&processed);
            if seen_units.contains(&key) {
                removed_dupes += 1;
                continue;
            }
            seen_units.insert(key);
            rebuilt.push(processed);
        }

        if !rebuilt.is_empty() {
            seen_lines.push(rebuilt.join(joiner));
        }
    }

    while seen_lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        seen_lines.pop();
    }

    let compressed = seen_lines.join("\n").trim().to_string();
    let compressed_tokens = estimate_tokens(&compressed);
    let reduction_pct = if original_tokens == 0 {
        0.0
    } else {
        ((original_tokens.saturating_sub(compressed_tokens)) as f64 / original_tokens as f64
            * 100.0)
            .max(0.0)
    };

    // Build the removed-redundancy summary.
    let mut removed_redundancy: Vec<String> = Vec::new();
    if removed_dupes > 0 {
        removed_redundancy.push(format!("Removed {removed_dupes} duplicate sentence(s)"));
    }
    if !removed_filler.is_empty() {
        removed_redundancy.push(format!("Removed filler: {}", removed_filler.join(", ")));
    }
    if compressed.lines().count() < original.lines().filter(|l| !l.trim().is_empty()).count() {
        removed_redundancy.push("Collapsed blank lines / whitespace".to_string());
    }
    if removed_redundancy.is_empty() {
        removed_redundancy.push("No redundancy detected".to_string());
    }

    dedup(&mut preserved);

    CompressionResult {
        mode: mode.as_str().to_string(),
        original,
        compressed,
        original_tokens,
        compressed_tokens,
        reduction_pct,
        estimated: true,
        preserved_constraints: preserved,
        removed_redundancy,
        conflicts: detect_conflicts(prompt),
        response_rules: response_rules(mode),
    }
}

fn dedup(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| seen.insert(s.to_lowercase()));
}

/// Pull the human-meaningful constraint text out of a preserved unit.
fn collect_constraints(unit: &str, out: &mut Vec<String>) {
    let trimmed = unit.trim_start_matches(['-', '*', ' ']).trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
}

/// Contradiction pairs: if a term from both sides appears, surface a conflict.
const CONFLICT_PAIRS: &[(&[&str], &[&str])] = &[
    (
        &[
            "minimal",
            "minimalist",
            "lightweight",
            "bare",
            "simple",
            "barebones",
        ],
        &[
            "huge",
            "animation-heavy",
            "feature-rich",
            "elaborate",
            "heavy",
            "flashy",
            "lots of features",
        ],
    ),
    (
        &["concise", "short", "terse", "brief", "minimal output"],
        &[
            "detailed",
            "verbose",
            "comprehensive",
            "exhaustive",
            "in-depth",
            "long",
        ],
    ),
    (
        &[
            "no dependencies",
            "zero dependencies",
            "dependency-free",
            "no libraries",
        ],
        &[
            "use a library",
            "add a library",
            "install ",
            "use the package",
        ],
    ),
    (
        &[
            "do not refactor",
            "no refactor",
            "don't refactor",
            "do not rewrite",
        ],
        &[
            "refactor everything",
            "rewrite everything",
            "rewrite the whole",
        ],
    ),
];

/// Detect possible contradictory instructions. Deterministic; never guesses a
/// resolution — it reports so the user can resolve.
pub fn detect_conflicts(prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut conflicts = Vec::new();
    for (a_terms, b_terms) in CONFLICT_PAIRS {
        let a_hit = a_terms.iter().find(|t| lower.contains(*t));
        let b_hit = b_terms.iter().find(|t| lower.contains(*t));
        if let (Some(a), Some(b)) = (a_hit, b_hit) {
            conflicts.push(format!(
                "Possible conflict: '{}' vs '{}'",
                a.trim(),
                b.trim()
            ));
        }
    }
    conflicts
}

/// The response-control block ("output discipline") for a mode.
pub fn response_rules(mode: CompressionMode) -> String {
    match mode {
        CompressionMode::Normal => [
            "Response rules:",
            "- Be clear and direct.",
            "- Avoid filler and repetition.",
            "- Do not claim work is complete unless verified.",
        ]
        .join("\n"),
        CompressionMode::Concise => [
            "Response rules:",
            "- Be direct. No filler.",
            "- No motivational language. No sugarcoating.",
            "- No claims of completion unless verified.",
            "- Do not paste full files unless needed.",
            "- Show changed files and commands only.",
            "- State failures clearly.",
            "- Keep the response short unless asked for detail.",
        ]
        .join("\n"),
        CompressionMode::Bare => [
            "Response rules (Bare Mode):",
            "- Be direct. Use the fewest words that preserve correctness.",
            "- Do not sugarcoat. Do not lie.",
            "- Do not pretend something is done if it is not done.",
            "- Do not add motivational filler.",
            "- Do not apologize unless there is a real failure.",
            "- Do not repeat the request. Do not explain obvious basics.",
            "- Do not paste entire files unless necessary.",
            "- Do not dump huge code blocks unless asked. Show only changed code.",
            "- Say what changed. Say what failed. Say what remains.",
            "- Ask only when blocked.",
            "- Prefer bullets over paragraphs.",
            "- Prefer exact commands over vague explanations.",
            "- Keep output minimal unless the user requests detail.",
        ]
        .join("\n"),
    }
}

/// Output-budget presets that generate response-size guidance (not enforcement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputBudgetPreset {
    Tiny,
    Short,
    #[default]
    Normal,
    Detailed,
}

impl OutputBudgetPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputBudgetPreset::Tiny => "tiny",
            OutputBudgetPreset::Short => "short",
            OutputBudgetPreset::Normal => "normal",
            OutputBudgetPreset::Detailed => "detailed",
        }
    }

    pub fn parse(s: &str) -> OutputBudgetPreset {
        match s.trim().to_lowercase().as_str() {
            "tiny" => OutputBudgetPreset::Tiny,
            "short" => OutputBudgetPreset::Short,
            "detailed" => OutputBudgetPreset::Detailed,
            _ => OutputBudgetPreset::Normal,
        }
    }

    /// Instruction text. Guidance only; real caps require provider support.
    pub fn to_instruction_block(&self) -> String {
        match self {
            OutputBudgetPreset::Tiny => "Output budget (Tiny): Keep response under 8 bullets. No paragraphs. No full files. Mention only changed files, commands run, failures, and next step.".to_string(),
            OutputBudgetPreset::Short => "Output budget (Short): Use concise bullets. Avoid repeated explanations. Show code snippets only when necessary.".to_string(),
            OutputBudgetPreset::Normal => "Output budget (Normal): Balanced detail. Explain only non-obvious decisions. Avoid filler.".to_string(),
            OutputBudgetPreset::Detailed => "Output budget (Detailed): You may explain reasoning and include relevant code. Still avoid filler and repetition.".to_string(),
        }
    }
}

/// Legacy fine-grained output-budget options (kept for the checkbox builder).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OutputBudget {
    pub concise: bool,
    pub no_repeat_unchanged_code: bool,
    pub summarize_in_bullets: bool,
    pub only_show_changed_files: bool,
    pub no_full_files_unless_necessary: bool,
    pub ask_before_large_rewrites: bool,
    pub target_tokens: Option<usize>,
}

impl OutputBudget {
    pub fn to_instruction_block(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if self.concise {
            lines.push("- Be concise.".into());
        }
        if self.no_repeat_unchanged_code {
            lines.push("- Do not repeat code unless it changed.".into());
        }
        if self.summarize_in_bullets {
            lines.push("- Summarize changes in bullet points.".into());
        }
        if self.only_show_changed_files {
            lines.push("- Only show files that changed.".into());
        }
        if self.no_full_files_unless_necessary {
            lines.push("- Do not paste full files unless necessary.".into());
        }
        if self.ask_before_large_rewrites {
            lines.push("- Ask before doing large rewrites.".into());
        }
        if let Some(t) = self.target_tokens {
            lines.push(format!(
                "- Keep output under roughly {t} tokens (soft target)."
            ));
        }
        if lines.is_empty() {
            return String::new();
        }
        format!("Output requirements:\n{}", lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_are_monotonic() {
        assert!(estimate_tokens("a much longer piece of text here") > estimate_tokens("short"));
    }

    #[test]
    fn removes_filler_but_keeps_intent() {
        let r = compress("I would like you to please basically just fix the bug.");
        assert!(r.compressed.to_lowercase().contains("fix the bug"));
        assert!(!r.compressed.to_lowercase().contains("basically"));
        assert!(!r.compressed.to_lowercase().contains("please"));
        assert!(r.compressed_tokens <= r.original_tokens);
    }

    #[test]
    fn preserves_constraints_commands_paths() {
        let p = "Please refactor.\nDo NOT rewrite the whole app.\nRun npm test.\nEdit src/auth/login.ts only.";
        let r = compress(p);
        assert!(r.compressed.contains("Do NOT rewrite the whole app."));
        assert!(r.compressed.contains("npm test"));
        assert!(r.compressed.contains("src/auth/login.ts"));
        assert!(r
            .preserved_constraints
            .iter()
            .any(|c| c.contains("Do NOT rewrite")));
    }

    #[test]
    fn preserves_quoted_strings_and_env_and_url() {
        let p =
            "set the header to \"X-Trace-Id\"\nuse DATABASE_URL\nfetch https://api.example.com/v1";
        let r = compress(p);
        assert!(r.compressed.contains("\"X-Trace-Id\""));
        assert!(r.compressed.contains("DATABASE_URL"));
        assert!(r.compressed.contains("https://api.example.com/v1"));
    }

    #[test]
    fn preserves_code_blocks() {
        let p = "do this\n```\nfn main() {  let x=1;  }\n```\nthanks";
        let r = compress(p);
        assert!(r.compressed.contains("fn main() {  let x=1;  }"));
    }

    #[test]
    fn dedupes_repeated_lines() {
        let r = compress("fix the login bug\nfix the login bug\nfix the login bug");
        assert_eq!(r.compressed.lines().count(), 1);
    }

    #[test]
    fn never_removes_safety_keywords() {
        let p = "do not change the schema. never delete data. keep the API stable.";
        let r = compress_with_mode(p, CompressionMode::Bare);
        let c = r.compressed.to_lowercase();
        assert!(c.contains("do not change"));
        assert!(c.contains("never delete"));
        assert!(c.contains("keep the api"));
    }

    #[test]
    fn detects_conflicts() {
        let r = compress("make it minimal. also add a huge animation-heavy UI.");
        assert!(!r.conflicts.is_empty());
        assert!(r.conflicts[0].to_lowercase().contains("minimal"));
    }

    #[test]
    fn bare_mode_has_strict_rules() {
        let r = compress_with_mode("fix it", CompressionMode::Bare);
        assert!(r.response_rules.contains("Bare Mode"));
        assert!(r.response_rules.contains("Do not sugarcoat"));
    }

    #[test]
    fn modes_differ_in_aggressiveness() {
        let p = "I really just want you to basically clean this up please.";
        let normal = compress_with_mode(p, CompressionMode::Normal);
        let concise = compress_with_mode(p, CompressionMode::Concise);
        // Concise strips "really/just/basically"; Normal keeps them.
        assert!(normal.compressed.to_lowercase().contains("basically"));
        assert!(!concise.compressed.to_lowercase().contains("basically"));
    }

    #[test]
    fn output_budget_presets_render() {
        assert!(OutputBudgetPreset::Tiny
            .to_instruction_block()
            .contains("8 bullets"));
        assert!(OutputBudgetPreset::Short
            .to_instruction_block()
            .contains("concise bullets"));
        assert!(OutputBudgetPreset::Detailed
            .to_instruction_block()
            .contains("explain reasoning"));
    }

    #[test]
    fn keeps_look_like_intact() {
        let r = compress_with_mode("make it look like the mockup", CompressionMode::Bare);
        assert!(r.compressed.to_lowercase().contains("look like"));
    }
}
