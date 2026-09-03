//! Concrete deterministic analyzers. Each is a small, independently testable
//! module implementing `intel::registry::Analyzer`.

pub mod retry_loop;
pub mod volume_anomaly;
