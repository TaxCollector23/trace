//! Local, rule-based secret detection and redaction.
//!
//! Trace never stores raw secrets. Detected values are redacted to a short
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
        p("gitlab_token", 6, r"glpat-[A-Za-z0-9\-_]{18,}"),
        p("aws_access_key", 4, r"AKIA[0-9A-Z]{16}"),
        // AWS secret access keys have no distinctive prefix, so we anchor on a
        // nearby `aws...secret...` label to avoid flagging random base64.
        p(
            "aws_secret_key",
            4,
            r#"(?i)aws[_\- ]?secret[_\- ]?access[_\- ]?key['"\s:=]+([A-Za-z0-9/+]{40})"#,
        ),
        p("google_api_key", 6, r"AIza[0-9A-Za-z\-_]{35}"),
        p("groq_api_key", 4, r"gsk_[A-Za-z0-9]{20,}"),
        p("stripe_key", 8, r"[sr]k_(?:live|test)_[A-Za-z0-9]{16,}"),
        p("slack_token", 9, r"xox[baprs]-[A-Za-z0-9\-]{10,}"),
        p("sendgrid_key", 5, r"SG\.[A-Za-z0-9\-_]{20,}\.[A-Za-z0-9\-_]{20,}"),
        p("npm_token", 7, r"npm_[A-Za-z0-9]{30,}"),
        p("twilio_account_sid", 6, r"AC[0-9a-fA-F]{32}"),
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
    let mut push = |secret_type: String, redacted_value: String| {
        let finding = SecretFinding {
            secret_type,
            redacted_value,
        };
        if !findings.contains(&finding) {
            findings.push(finding);
        }
    };
    for pattern in PATTERNS.iter() {
        for m in pattern.regex.find_iter(text) {
            push(
                pattern.secret_type.to_string(),
                redact(m.as_str(), pattern.keep),
            );
        }
    }
    // Supplemental patterns from the versioned rule pack (additive only).
    for (secret_type, keep, re) in crate::rules_pack::active().compiled_secret_patterns() {
        for m in re.find_iter(text) {
            push(secret_type.clone(), redact(m.as_str(), keep));
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
    fn detects_expanded_provider_set() {
        // Tokens split with concat! so no contiguous secret literal is stored
        // in source (GitHub push protection); runtime value is unchanged.
        for (text, want) in [
            (
                concat!("STRIPE_KEY=sk", "_live_abcdefghijklmnopqrstuvwx"),
                "stripe_key",
            ),
            (
                concat!("SLACK_TOKEN=xox", "b-123456789012-abcdefghijklmnop"),
                "slack_token",
            ),
            (
                concat!("GITLAB_TOKEN=glp", "at-abcdefghijklmnopqrst"),
                "gitlab_token",
            ),
            (
                concat!("NPM_TOKEN=npm", "_abcdefghijklmnopqrstuvwxyz0123456789"),
                "npm_token",
            ),
            (
                concat!(
                    "SG",
                    ".abcdefghijklmnopqrstuv.abcdefghijklmnopqrstuvwxyz0123456789AB"
                ),
                "sendgrid_key",
            ),
            (
                concat!(
                    "aws_secret_access_key = wJalrXUtnFEMI",
                    "/K7MDENG/bPxRfiCYEXAMPLEKEY"
                ),
                "aws_secret_key",
            ),
        ] {
            let found = scan_text(text);
            assert!(
                found.iter().any(|f| f.secret_type == want),
                "expected {want} in {text}, got {found:?}"
            );
            assert!(found
                .iter()
                .all(|f| f.redacted_value.ends_with("...redacted")));
        }
    }

    #[test]
    fn env_filename_detection() {
        assert!(is_env_like_filename("config/.env.local"));
        assert!(is_env_like_filename("id_rsa"));
        assert!(!is_env_like_filename("src/main.rs"));
    }
}
