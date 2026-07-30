//! Local, rule-based secret detection and redaction.
//!
//! TraceGuard never stores raw secrets. Detected values are redacted to a short
//! prefix plus `...redacted` before they leave this module.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A detected secret. `redacted_value` is always safe to persist and display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretFinding {
    pub secret_type: String,
    pub redacted_value: String,
}

struct Pattern {
    secret_type: &'static str,
    /// Number of leading characters to keep before `...redacted`.
    keep: usize,
    regex: Regex,
}

static PATTERNS: Lazy<Vec<Pattern>> = Lazy::new(|| {
    let p = |secret_type: &'static str, keep: usize, re: &str| Pattern {
        secret_type,
        keep,
        regex: Regex::new(re).expect("valid secret regex"),
    };
    vec![
        p("anthropic_api_key", 7, r"sk-ant-[A-Za-z0-9\-_]{20,}"),
        p("openai_api_key", 6, r"sk-(?:proj-)?[A-Za-z0-9]{20,}"),
        p("github_token", 4, r"gh[pousr]_[A-Za-z0-9]{20,}"),
        p("aws_access_key", 4, r"AKIA[0-9A-Z]{16}"),
        p("google_api_key", 6, r"AIza[0-9A-Za-z\-_]{35}"),
        p("groq_api_key", 4, r"gsk_[A-Za-z0-9]{20,}"),
        p(
            "private_ssh_key",
            10,
            r"-----BEGIN (?:RSA |OPENSSH |EC |DSA )?PRIVATE KEY-----",
        ),
        p(
            "jwt",
            6,
            r"eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+",
        ),
        p(
            "database_url",
            10,
            r#"(?:postgres|postgresql|mysql|mongodb(?:\+srv)?)://[^\s'"]+"#,
        ),
        p(
            "generic_bearer_token",
            7,
            r"[Bb]earer\s+[A-Za-z0-9\-_\.=]{20,}",
        ),
    ]
});

/// Redact a matched value to a short, non-recoverable form.
fn redact(value: &str, keep: usize) -> String {
    let prefix: String = value.chars().take(keep).collect();
    format!("{prefix}...redacted")
}

/// Scan arbitrary text and return de-duplicated findings.
pub fn scan_text(text: &str) -> Vec<SecretFinding> {
    let mut findings: Vec<SecretFinding> = Vec::new();
    for pattern in PATTERNS.iter() {
        for m in pattern.regex.find_iter(text) {
            let finding = SecretFinding {
                secret_type: pattern.secret_type.to_string(),
                redacted_value: redact(m.as_str(), pattern.keep),
            };
            if !findings.contains(&finding) {
                findings.push(finding);
            }
        }
    }
    findings
}

/// Returns true when the file name itself indicates an environment/secret file.
pub fn is_env_like_filename(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let name = normalized.rsplit('/').next().unwrap_or(&normalized);
    name == ".env"
        || name.starts_with(".env.")
        || name == "id_rsa"
        || name == "secrets.json"
        || name.ends_with(".pem")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_redacts_anthropic_key() {
        let text = "ANTHROPIC_API_KEY=sk-ant-abcdefghij1234567890ABCDEF";
        let found = scan_text(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].secret_type, "anthropic_api_key");
        assert!(found[0].redacted_value.ends_with("...redacted"));
        assert!(!found[0].redacted_value.contains("1234567890"));
    }

    #[test]
    fn detects_aws_key() {
        let found = scan_text("aws_key = AKIAIOSFODNN7EXAMPLE");
        assert!(found.iter().any(|f| f.secret_type == "aws_access_key"));
    }

    #[test]
    fn clean_text_has_no_findings() {
        assert!(scan_text("just some normal source code; let x = 1;").is_empty());
    }

    #[test]
    fn env_filename_detection() {
        assert!(is_env_like_filename("config/.env.local"));
        assert!(is_env_like_filename("id_rsa"));
        assert!(!is_env_like_filename("src/main.rs"));
    }
}
