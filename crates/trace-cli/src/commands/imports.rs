//! `trc import` — import agent-owned usage metadata into Trace.

use anyhow::Result;
use trace_core::importers::{self, ImportContext, ImportReport};
use trace_core::{paths, Store};

pub fn run(agent: &str, json: bool) -> Result<()> {
    let db_path = paths::database_path()?;
    let store = Store::open(&db_path)?;
    let context = ImportContext::from_environment()?;
    let reports = if agent == "all" {
        importers::import_all(&store, &context)?
    } else {
        vec![importers::import_agent(&store, &context, agent)?]
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        print_reports(&reports);
    }
    Ok(())
}

fn print_reports(reports: &[ImportReport]) {
    println!("Trace import — local metadata only");
    for report in reports {
        println!(
            "  {:<9} files {}/{} · events {} seen, {} imported, {} duplicate · parse errors {}",
            report.agent,
            report.files_scanned,
            report.files_found,
            report.events_seen,
            report.events_imported,
            report.duplicates,
            report.parse_errors,
        );
        if let Some(last) = &report.last_event_at {
            println!("             last event {last}");
        }
        for source in &report.sources {
            println!("             {source}");
        }
    }
}
