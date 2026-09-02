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
        p(
            "sendgrid_key",
            5,
            r"SG\.[A-Za-z0-9\-_]{20,}\.[A-Za-z0-9\-_]{20,}",
        ),
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

/// Scrub every detected secret out of `text`, replacing each match in place
/// with its redacted form (`prefix...redacted`). Returns the sanitized string.
///
/// This is the **storage-boundary** filter: route any text through it *before*
/// it is written to disk or the database (`commands.command`, `stdout.log` /
/// `stderr.log`, `diff.patch.gz`) so the raw secret never lands in persistent
/// storage. It is a superset of [`scan_text`] — same patterns, but it rewrites
/// the text instead of only reporting findings. When `text` contains no secret
/// it is returned byte-for-byte unchanged.
pub fn redact_text(text: &str) -> String {
    let mut out = text.to_string();
    // Built-in patterns first, then the versioned rule-pack patterns, so the
    // scrubber covers exactly what the scanner detects.
    for pattern in PATTERNS.iter() {
        out = replace_all_redacted(&out, &pattern.regex, pattern.keep);
    }
    for (_secret_type, keep, re) in crate::rules_pack::active().compiled_secret_patterns() {
        out = replace_all_redacted(&out, &re, keep);
    }
    out
}

/// Replace every match of `re` in `text` with its redacted form.
fn replace_all_redacted(text: &str, re: &Regex, keep: usize) -> String {
    re.replace_all(text, |caps: &regex::Captures| redact(&caps[0], keep))
        .into_owned()
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
    fn redact_text_scrubs_secrets_but_keeps_surrounding_text() {
        let raw = concat!(
            "export ANTHROPIC_API_KEY=sk-ant-",
            "abcdefghij1234567890ABCDEF"
        );
        let body = concat!("sk-ant-", "abcdefghij1234567890ABCDEF");
        let scrubbed = redact_text(raw);
        assert!(!scrubbed.contains(body), "leaked secret: {scrubbed}");
        assert!(scrubbed.contains("redacted"));
        // Non-secret context is preserved.
        assert!(scrubbed.starts_with("export ANTHROPIC_API_KEY="));
    }

    #[test]
    fn redact_text_is_a_noop_on_clean_text() {
        let clean = "let total = subtotal + tax; // no secrets here";
        assert_eq!(redact_text(clean), clean);
    }

    #[test]
    fn redact_text_scrubs_multiple_and_mixed_secrets() {
        let aws = concat!("AKIA", "IOSFODNN7EXAMPLE");
        let gh = concat!("ghp", "_abcdefghijklmnopqrstuvwx0123");
        let raw = format!("key1={aws} key2={gh}");
        let scrubbed = redact_text(&raw);
        assert!(!scrubbed.contains(aws));
        assert!(!scrubbed.contains(gh));
    }

    #[test]
    fn env_filename_detection() {
        assert!(is_env_like_filename("config/.env.local"));
        assert!(is_env_like_filename("id_rsa"));
        assert!(!is_env_like_filename("src/main.rs"));
    }

    /// Core safety invariant: for every built-in secret type, the persisted
    /// `redacted_value` must NEVER contain the full secret body. Inputs are
    /// assembled with `concat!` so no contiguous secret literal exists in
    /// source (GitHub push protection); the runtime string is unchanged.
    #[test]
    fn redaction_never_leaks_the_full_secret() {
        // (input containing the secret, secret body that must not survive, type)
        let cases: &[(&str, &str, &str)] = &[
            (
                concat!("ANTHROPIC_API_KEY=sk-ant-", "abcdefghij1234567890ABCDEFxyz"),
                concat!("sk-ant-", "abcdefghij1234567890ABCDEFxyz"),
                "anthropic_api_key",
            ),
            (
                concat!("OPENAI_API_KEY=sk-", "abcdefghijklmnopqrstuvwx012345"),
                concat!("sk-", "abcdefghijklmnopqrstuvwx012345"),
                "openai_api_key",
            ),
            (
                concat!("GITHUB_TOKEN=ghp", "_abcdefghijklmnopqrstuvwx0123"),
                concat!("ghp", "_abcdefghijklmnopqrstuvwx0123"),
                "github_token",
            ),
            (
                concat!("GITLAB_TOKEN=glp", "at-abcdefghijklmnopqrst0123"),
                concat!("glp", "at-abcdefghijklmnopqrst0123"),
                "gitlab_token",
            ),
            (
                concat!("AWS_ACCESS_KEY_ID=AKIA", "IOSFODNN7EXAMPLE"),
                concat!("AKIA", "IOSFODNN7EXAMPLE"),
                "aws_access_key",
            ),
            (
                concat!(
                    "aws_secret_access_key = wJalrXUtnFEMI",
                    "/K7MDENG/bPxRfiCYEXAMPLEKEY"
                ),
                concat!("wJalrXUtnFEMI", "/K7MDENG/bPxRfiCYEXAMPLEKEY"),
                "aws_secret_key",
            ),
            (
                concat!("GOOGLE_API_KEY=AIza", "SyC8bQZ9xY2wV4uT6rS0pN1mK3jH5gF7dE9"),
                concat!("AIza", "SyC8bQZ9xY2wV4uT6rS0pN1mK3jH5gF7dE9"),
                "google_api_key",
            ),
            (
                concat!("GROQ_API_KEY=gsk", "_abcdefghijklmnopqrstuvwx0123"),
                concat!("gsk", "_abcdefghijklmnopqrstuvwx0123"),
                "groq_api_key",
            ),
            (
                concat!("STRIPE_KEY=sk", "_live_abcdefghijklmnopqrst"),
                concat!("sk", "_live_abcdefghijklmnopqrst"),
                "stripe_key",
            ),
            (
                concat!("SLACK_TOKEN=xox", "b-123456789012-abcdefghijkl"),
                concat!("xox", "b-123456789012-abcdefghijkl"),
                "slack_token",
            ),
            (
                concat!(
                    "SG",
                    ".abcdefghijklmnopqrstuv.abcdefghijklmnopqrstuvwxyz01234"
                ),
                concat!(
                    "SG",
                    ".abcdefghijklmnopqrstuv.abcdefghijklmnopqrstuvwxyz01234"
                ),
                "sendgrid_key",
            ),
            (
                concat!("NPM_TOKEN=npm", "_abcdefghijklmnopqrstuvwxyz0123456789"),
                concat!("npm", "_abcdefghijklmnopqrstuvwxyz0123456789"),
                "npm_token",
            ),
            (
                concat!("TWILIO_SID=AC", "0123456789abcdef0123456789abcdef"),
                concat!("AC", "0123456789abcdef0123456789abcdef"),
                "twilio_account_sid",
            ),
            (
                concat!("-----BEGIN ", "OPENSSH PRIVATE KEY-----"),
                concat!("-----BEGIN ", "OPENSSH PRIVATE KEY-----"),
                "private_ssh_key",
            ),
            (
                concat!(
                    "token=eyJ",
                    "hbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6y"
                ),
                concat!(
                    "eyJ",
                    "hbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6y"
                ),
                "jwt",
            ),
            (
                concat!("DATABASE_URL=postgres", "://user:pass@localhost:5432/mydb"),
                concat!("postgres", "://user:pass@localhost:5432/mydb"),
                "database_url",
            ),
            (
                concat!("Authorization: Bearer ", "abcdefghij1234567890KLMNOPQRST"),
                "abcdefghij1234567890KLMNOPQRST",
                "generic_bearer_token",
            ),
        ];

        for (input, body, want_type) in cases {
            let found = scan_text(input);
            let hit = found
                .iter()
                .find(|f| f.secret_type == *want_type)
                .unwrap_or_else(|| panic!("expected {want_type} to be detected in {input:?}"));
            // The redaction must not carry the full secret body through.
            assert!(
                !hit.redacted_value.contains(body),
                "{want_type}: redacted_value {:?} leaked the full secret",
                hit.redacted_value
            );
            // And it must be recognizably redacted.
            assert!(
                hit.redacted_value.ends_with("...redacted"),
                "{want_type}: redacted_value {:?} not marked redacted",
                hit.redacted_value
            );
            // No finding for this input may leak the full body either.
            for f in &found {
                assert!(
                    !f.redacted_value.contains(body),
                    "{}: redacted_value {:?} leaked the full secret",
                    f.secret_type,
                    f.redacted_value
                );
            }
        }
    }

    #[test]
    fn no_false_positive_on_placeholders_and_prose() {
        // A plain sentence contains no secret-shaped tokens.
        assert!(scan_text("Please rotate the API key before Friday's deploy.").is_empty());
        // A short, clearly-templated bearer placeholder stays below the
        // token-length threshold and is not flagged.
        assert!(scan_text("Authorization: Bearer <your-token-here>").is_empty());
        // NOTE (documented behavior): a long placeholder such as
        // `Bearer your-token-here-example` (23 body chars) is, by pattern alone,
        // indistinguishable from a real token and IS conservatively flagged. We
        // assert that here so the behavior is pinned rather than silently drifting.
        assert!(scan_text("Authorization: Bearer your-token-here-example")
            .iter()
            .any(|f| f.secret_type == "generic_bearer_token"));
    }
}
