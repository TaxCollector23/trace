//! Shared importer interface and import accounting.

use anyhow::Result;

use crate::Store;

/// Per-run import accounting. Counts are about source processing, not token
/// totals; a duplicate stream record is reported as updated, never counted as
/// a second usage event.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportReport {
    pub agent: String,
    pub files_seen: i64,
    pub lines_read: i64,
    pub events_inserted: i64,
    pub events_updated: i64,
    pub malformed_lines: i64,
    pub last_event_at: Option<String>,
}

/// Outcome of the canonical `(agent, session, message)` upsert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageUpsertResult {
    Inserted,
    Updated,
}

impl ImportReport {
    pub fn merge(&mut self, other: &Self) {
        if self.agent.is_empty() {
            self.agent = other.agent.clone();
        }
        self.files_seen += other.files_seen;
        self.lines_read += other.lines_read;
        self.events_inserted += other.events_inserted;
        self.events_updated += other.events_updated;
        self.malformed_lines += other.malformed_lines;
        if let Some(candidate) = other.last_event_at.as_deref() {
            let replace = self
                .last_event_at
                .as_deref()
                .map_or(true, |current| candidate > current);
            if replace {
                self.last_event_at = Some(candidate.to_string());
            }
        }
    }
}

/// Contract for a passive, local usage importer.
///
/// Adding another agent should require a new implementation plus sanitized
/// fixtures. The store owns dedupe and offset persistence so all importers
/// share the same idempotency rules.
pub trait UsageImporter {
    /// Stable agent identifier stored in `usage_events.agent`.
    fn agent(&self) -> &'static str;

    /// Import newly appended source lines into the canonical store.
    fn import(&self, store: &Store) -> Result<ImportReport>;
}
