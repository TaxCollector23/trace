//! `Incident` — the wire shape `apps/web/src/data.ts`'s `Incident` interface
//! expects from `GET /api/runs/:id/incidents`.
//!
//! This module only defines the `Incident` shape itself. The logic that
//! groups `Signal`s into `Incident`s lives in [`crate::intel::correlation`] —
//! see that module's docs for the grouping/escalation policy (Wave 2:
//! signals are correlated by evidence overlap or shared target + tight time
//! window, not escalated 1:1 by severity as an earlier version of this file
//! did).

use serde::{Deserialize, Serialize};

use crate::policy::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: String,
    pub run_id: String,
    pub severity: Severity,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub signal_ids: Vec<String>,
    /// Union of the grouped signals' `evidence_event_ids` — every event id
    /// that backs this incident, sorted for determinism. Additive relative to
    /// the `apps/web` `Incident` TypeScript interface (extra JSON fields are
    /// ignored by consumers that don't know about them yet).
    pub evidence: Vec<String>,
    pub first_seen: String,
    pub last_seen: String,
}
