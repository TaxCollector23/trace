//! Timestamp helpers. All stored times are UTC RFC 3339 strings.

use chrono::Utc;

/// Current UTC time formatted as RFC 3339 (e.g. `2026-06-14T20:30:00Z`).
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
