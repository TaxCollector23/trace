//! `trc status` — a compact, actionable snapshot for humans and scripts.

use anyhow::Result;
use serde_json::json;

use crate::{daemon_ctl, project};

pub fn run(json_output: bool) -> Result<()> {
    let daemon = daemon_ctl::status()?;
    let integrations = trace_core::integrations::detect_connections();
    let connected = integrations.iter().filter(|item| item.connected).count();
    let project_root = std::env::current_dir()
        .ok()
        .and_then(|cwd| project::find_project_root(&cwd));

    if json_output {
        let value = json!({
            "version": trace_core::VERSION,
            "daemon": {
                "running": daemon.running,
                "port": daemon.port,
                "pid": daemon.pid,
                "started_at": daemon.started_at,
                "database": daemon.db_path,
            },
            "integrations": {
                "connected": connected,
                "total": integrations.len(),
                "agents": integrations.iter().map(|item| json!({
                    "id": item.id,
                    "name": item.display_name,
                    "connected": item.connected,
                    "how": item.how,
                })).collect::<Vec<_>>(),
            },
            "project": project_root.map(|path| path.display().to_string()),
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("Trace status");
    match daemon.port {
        Some(port) if daemon.running => println!("daemon: connected at http://127.0.0.1:{port}"),
        Some(port) => println!("daemon: not responding (last known port {port})"),
        None => println!("daemon: not running — use `trc dashboard` to start it"),
    }
    println!("agents: {connected}/{} connected", integrations.len());
    if let Some(root) = project_root {
        println!("folder: {}", root.display());
    } else {
        println!("folder: not a Trace project — use `trc init` here");
    }
    if connected < integrations.len() {
        println!("next: trc integrations install all");
    }
    println!("open: trc dashboard");
    Ok(())
}
