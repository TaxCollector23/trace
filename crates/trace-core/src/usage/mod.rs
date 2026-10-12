//! Canonical usage events and importer contracts.
//!
//! Usage importers deliberately persist only accounting metadata. Transcript
//! prompt/response content is never part of these types or the storage API.

pub mod claude_code;
pub mod importer;

use serde::{Deserialize, Serialize};

pub use claude_code::ClaudeCodeImporter;
pub use importer::{ImportReport, UsageImporter};

// Canonical event/storage shapes live in the existing shared model module so
// the importer and the legacy API cannot drift into two row contracts.
pub use crate::models::{ImportSourceStatus, NewUsageEvent, UsageCostSource, UsageEvent};

/// The supported first consumer's grouping dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageGroup {
    Model,
    Session,
    Day,
    Agent,
    Project,
}

impl UsageGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Session => "session",
            Self::Day => "day",
            Self::Agent => "agent",
            Self::Project => "project",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "model" => Some(Self::Model),
            "session" => Some(Self::Session),
            "day" => Some(Self::Day),
            "agent" => Some(Self::Agent),
            "project" => Some(Self::Project),
            _ => None,
        }
    }
}

impl Default for UsageGroup {
    fn default() -> Self {
        Self::Model
    }
}

/// Totals for a grouping row or for the entire imported data set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageTotals {
    pub event_count: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub cost_source: String,
    pub unavailable_cost_events: i64,
    pub subagent_events: i64,
}

/// One row in a usage breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageBreakdown {
    pub key: String,
    #[serde(flatten)]
    pub totals: UsageTotals,
}

/// The response shared by `trc usage` and `GET /api/usage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageResponse {
    pub by: String,
    pub since: Option<String>,
    pub totals: UsageTotals,
    pub rows: Vec<UsageBreakdown>,
    pub import: ImportReport,
}
