//! `trc demo <scenario>` — generate a fully realistic, deterministic run
//! directly into the real local database, so `trc dashboard` immediately has
//! something real-looking to click through without needing an actual coding
//! agent running.
//!
//! This answers the product owner's literal question ("how can I test the
//! dashboard again without needing a real coding agent running?"). It writes
//! through the same [`trace_core::Store`] API a real `trc run` would use
//! (see `trace_core::demo`), under a dedicated `trace-demo` project so it
//! never collides with a user's real registered projects, and every row it
//! creates is tagged synthetic so it is easy to tell apart from real
//! telemetry later. Cleanup is `trc reset --local-data`, which already
//! purges every run regardless of origin.
//!
//! This command only ever runs when the user explicitly types `trc demo
//! <scenario>` — nothing else in Trace calls into `trace_core::demo`.

use anyhow::{Context, Result};
use trace_core::demo::{find_scenario, run_scenario, SCENARIOS};
use trace_core::models::NewProject;
use trace_core::{paths, Store};

use crate::colors;

/// Dedicated project every demo run is registered under. A non-filesystem
/// path so it can never collide with a real project's (always absolute,
/// canonicalized) registered path.
const DEMO_PROJECT_NAME: &str = "trace-demo";
const DEMO_PROJECT_PATH: &str = "trace-demo";

pub fn run(scenario: &str, seed: u64) -> Result<()> {
    if scenario == "list" {
        print_scenarios();
        return Ok(());
    }

    let Some(info) = find_scenario(scenario) else {
        println!(
            "{} unknown demo scenario '{scenario}'.\n",
            colors::red("error:")
        );
        print_scenarios();
        anyhow::bail!("run `trc demo list` to see available scenarios");
    };

    let db_path = paths::database_path()?;
    let store = Store::open(&db_path).context("opening the local database")?;

    let project = store
        .upsert_project(&NewProject {
            name: DEMO_PROJECT_NAME.to_string(),
            path: DEMO_PROJECT_PATH.to_string(),
            config_path: format!("{DEMO_PROJECT_PATH}/.trace/config.toml"),
        })
        .context("registering the dedicated trace-demo project")?;

    let run = run_scenario(&store, &project.id, scenario, seed)
        .with_context(|| format!("generating demo scenario '{scenario}'"))?;

    println!(
        "{} generated demo run {} ({})",
        colors::green("done:"),
        colors::bold(&run.id),
        info.description
    );
    println!("  scenario: {scenario}  (seed {seed})");
    println!("  project:  {DEMO_PROJECT_NAME}");
    println!("\nOpen the dashboard to see it: `trc dashboard`");
    println!("Remove it later with: `trc reset --local-data`");
    Ok(())
}

fn print_scenarios() {
    println!("{}", colors::bold("Available demo scenarios:"));
    for s in SCENARIOS {
        println!("  {:<20} {}", s.name, s.description);
    }
    println!("\nUsage: trc demo <scenario> [--seed <N>]");
}
