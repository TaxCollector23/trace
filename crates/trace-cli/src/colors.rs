//! Minimal ANSI coloring for CLI output. No external crate: a handful of
//! wrap-in-escape-codes helpers, gated on the same tty + NO_COLOR rules as
//! the startup banner. Color is applied to already-padded text so fixed-width
//! `{:<N}` alignment in query.rs is unaffected.

use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
}

fn wrap(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn dim(s: &str) -> String {
    wrap("2", s)
}
pub fn bold(s: &str) -> String {
    wrap("1", s)
}
pub fn green(s: &str) -> String {
    wrap("32", s)
}
pub fn yellow(s: &str) -> String {
    wrap("33", s)
}
pub fn red(s: &str) -> String {
    wrap("31", s)
}
pub fn brand(s: &str) -> String {
    if enabled() {
        format!("\x1b[38;2;124;123;251m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn status_fn(s: &str) -> fn(&str) -> String {
    match s {
        "completed" | "passed" | "allow" | "created" => green,
        "warn" | "blocked_warn" | "require_approval" => yellow,
        "failed" | "blocked" | "block" => red,
        "running" => brand,
        _ => |s: &str| s.to_string(),
    }
}

/// Color a run/command status by its meaning. Falls back to plain text.
pub fn status(s: &str) -> String {
    status_fn(s)(s)
}

/// Left-pad `raw` to `width` (matching a plain `{:<width}` format), then
/// color the padded text by `raw`'s meaning. Classify-then-pad-then-wrap, in
/// that order — coloring a pre-padded string breaks the match on trailing
/// whitespace.
pub fn status_padded(raw: &str, width: usize) -> String {
    status_fn(raw)(&format!("{raw:<width$}"))
}
