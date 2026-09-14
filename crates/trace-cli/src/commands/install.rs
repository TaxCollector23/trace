//! `trc install <target>` — the friendly, top-level installer.
//!
//! Today the only target is `agents`, which wires Trace into every supported
//! coding agent (Claude Code, Cursor, Windsurf, Codex) and starts the local
//! daemon so live review works immediately. It's a thin, well-labeled wrapper
//! over the same idempotent installer behind `trc integrations install all`,
//! surfaced as a primary onboarding command.

use anyhow::Result;

use crate::commands::hook_install;
use crate::daemon_ctl;

pub fn run(target: &str) -> Result<()> {
    match target {
        "agents" | "agent" | "all" => install_agents(),
        other => {
            anyhow::bail!(
                "unknown install target '{other}'. Try `trc install agents` to connect your coding agents."
            );
        }
    }
}

fn install_agents() -> Result<()> {
    // Live review needs the daemon; start it now so the connection is complete
    // the moment an agent restarts. A failure here is non-fatal — the hooks/MCP
    // still install, they just won't stream live review until the daemon runs.
    let port = daemon_ctl::ensure_running().ok();

    // Install/patch every supported agent (idempotent, with backups).
    hook_install::install("all")?;

    if let Some(port) = port {
        println!("dashboard: http://127.0.0.1:{port}");
    }
    println!("agents connected");
    Ok(())
}
