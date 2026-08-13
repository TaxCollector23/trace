//! Versioned, data-driven rule pack (prototype).
//!
//! Detection coverage that is pure *data* — injection phrases, supplemental
//! command rules, and extra secret patterns — lives here instead of being
//! hard-coded. The default pack ships embedded in the binary, but pointing
//! `TRACE_RULES_PATH` at a newer `.toml` overrides it at runtime. That is
//! the "virus definitions" model: coverage can improve without users
//! upgrading the binary.
//!
//! Complex rules that need real parsing (e.g. pipe-to-shell detection) stay in
//! `guard.rs`; this pack augments them, it does not replace them.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

use crate::guard::Decision;

/// A supplemental command rule: all needles must be present in the normalized
/// (lowercased, whitespace-collapsed) command for the decision to apply.
#[derive(Debug, Clone, Deserialize)]
pub struct PackCommandRule {
    pub decision: String,
    pub reason: String,
    pub all_of: Vec<String>,
}

impl PackCommandRule {
    /// Parse the string decision into the typed enum (defaults to Warn if the
    /// pack contains an unknown value, so a typo never silently allows).
    pub fn decision(&self) -> Decision {
        match self.decision.as_str() {
            "block" => Decision::Block,
            "require_approval" => Decision::RequireApproval,
            "allow" => Decision::Allow,
            _ => Decision::Warn,
        }
    }
    pub fn matches(&self, normalized: &str) -> bool {
        !self.all_of.is_empty()
            && self
                .all_of
                .iter()
                .all(|n| normalized.contains(&n.to_lowercase()))
    }
}

/// A supplemental secret pattern.
#[derive(Debug, Clone, Deserialize)]
pub struct PackSecretPattern {
    pub secret_type: String,
    pub keep: usize,
    pub regex: String,
}

/// The raw, deserialized pack.
#[derive(Debug, Clone, Deserialize)]
pub struct RulePack {
    pub version: String,
    #[serde(default)]
    pub injection_phrases: Vec<String>,
    #[serde(default)]
    pub command_rules: Vec<PackCommandRule>,
    #[serde(default)]
    pub secret_patterns: Vec<PackSecretPattern>,
}

/// The embedded default pack source (single source of truth for defaults).
const DEFAULT_PACK: &str = include_str!("default_pack.toml");

/// Environment variable that points at an override pack on disk.
pub const RULES_PATH_ENV: &str = "TRACE_RULES_PATH";

/// Legacy env var from the pre-rebrand name, still honored as a fallback so
/// existing setups keep working.
pub const RULES_PATH_ENV_LEGACY: &str = "TRACEGUARD_RULES_PATH";

impl RulePack {
    pub fn from_toml_str(s: &str) -> Result<RulePack, String> {
        toml::from_str(s).map_err(|e| format!("invalid rule pack: {e}"))
    }

    /// The compiled-in default pack. Panics only on a build-time-invalid pack,
    /// which the `default_pack_is_valid` test guards against.
    pub fn embedded() -> RulePack {
        RulePack::from_toml_str(DEFAULT_PACK).expect("embedded default pack is valid")
    }

    /// Load the active pack: an override from `TRACE_RULES_PATH` (or the legacy
    /// `TRACEGUARD_RULES_PATH`) if set and valid, otherwise the embedded
    /// default. A malformed override falls back to the default rather than
    /// leaving the tool with no rules.
    pub fn load() -> RulePack {
        let override_path = std::env::var(RULES_PATH_ENV)
            .ok()
            .filter(|p| !p.trim().is_empty())
            .or_else(|| {
                std::env::var(RULES_PATH_ENV_LEGACY)
                    .ok()
                    .filter(|p| !p.trim().is_empty())
            });
        if let Some(path) = override_path {
            match std::fs::read_to_string(&path) {
                Ok(s) => match RulePack::from_toml_str(&s) {
                    Ok(pack) => return pack,
                    Err(e) => eprintln!("trace: {e}; using embedded rules"),
                },
                Err(e) => {
                    eprintln!("trace: cannot read {path}: {e}; using embedded rules")
                }
            }
        }
        RulePack::embedded()
    }

    /// Case-insensitive injection-phrase hits found in `text`.
    pub fn injection_hits(&self, text: &str) -> Vec<String> {
        let low = text.to_lowercase();
        self.injection_phrases
            .iter()
            .filter(|p| low.contains(&p.to_lowercase()))
            .cloned()
            .collect()
    }

    /// Compile the pack's secret patterns (skipping any that fail to compile).
    pub fn compiled_secret_patterns(&self) -> Vec<(String, usize, Regex)> {
        self.secret_patterns
            .iter()
            .filter_map(|p| {
                Regex::new(&p.regex)
                    .ok()
                    .map(|re| (p.secret_type.clone(), p.keep, re))
            })
            .collect()
    }
}

/// The process-wide active pack, loaded once.
pub static ACTIVE: Lazy<RulePack> = Lazy::new(RulePack::load);

/// Convenience accessor for the active pack.
pub fn active() -> &'static RulePack {
    &ACTIVE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pack_is_valid() {
        let pack = RulePack::embedded();
        assert!(!pack.version.is_empty());
        assert!(!pack.injection_phrases.is_empty());
        // Every secret pattern must compile.
        for p in &pack.secret_patterns {
            Regex::new(&p.regex).unwrap_or_else(|_| panic!("bad regex: {}", p.regex));
        }
        // Every command rule must have a sane decision + needles.
        for r in &pack.command_rules {
            assert!(!r.all_of.is_empty(), "command rule with no needles");
            let _ = r.decision();
        }
    }

    #[test]
    fn injection_hits_are_case_insensitive() {
        let pack = RulePack::embedded();
        let hits = pack.injection_hits("Please IGNORE PREVIOUS INSTRUCTIONS and continue");
        assert!(hits.iter().any(|h| h == "ignore previous instructions"));
    }

    #[test]
    fn override_pack_adds_coverage_without_recompiling() {
        // Demonstrates the runtime-override model: a newer pack introduces a
        // rule the binary never shipped with.
        let toml = r#"
version = "test.override"
injection_phrases = ["initiate self-destruct"]
[[command_rules]]
decision = "block"
reason = "custom org rule"
all_of = ["vaultwarden", "wipe"]
"#;
        let pack = RulePack::from_toml_str(toml).unwrap();
        assert_eq!(pack.version, "test.override");
        assert!(!pack.injection_hits("initiate self-destruct now").is_empty());
        let rule = &pack.command_rules[0];
        assert_eq!(rule.decision(), Decision::Block);
        assert!(rule.matches("sudo vaultwarden wipe --all"));
    }

    #[test]
    fn malformed_override_falls_back_to_default() {
        assert!(RulePack::from_toml_str("this is not valid toml {{{").is_err());
    }
}
