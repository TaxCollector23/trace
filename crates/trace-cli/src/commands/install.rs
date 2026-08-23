//! `trace install <target>` — the friendly, top-level installer.
//!
//! Today the only target is `agents`, which wires Trace into every supported
//! coding agent (Claude Code, Cursor, Windsurf, Codex) and starts the local
//! daemon so live review works immediately. It's a thin, well-labeled wrapper
//! over the same idempotent installer behind `trace integrations install all`,
//! surfaced as a primary onboarding command.

use anyhow::Result;

use crate::colors;
use crate::commands::{hook_install, integrations};
use crate::daemon_ctl;

pub fn run(target: &str) -> Result<()> {
    match target {
        "agents" | "agent" | "all" => install_agents(),
        other => {
            anyhow::bail!(
                "unknown install target '{other}'. Try `trace install agents` to connect your coding agents."
            );
        }
    }
}

fn install_agents() -> Result<()> {
    // Live review needs the daemon; start it now so the connection is complete
    // the moment an agent restarts. A failure here is non-fatal — the hooks/MCP
    // still install, they just won't stream live review until the daemon runs.
    match daemon_ctl::ensure_running() {
        Ok(port) => println!(
            "{} daemon running on http://127.0.0.1:{port}\n",
            colors::green("✓")
        ),
        Err(e) => println!(
            "{} couldn't start the daemon ({e}). Agents will still connect; run `trace daemon start` later for live review.\n",
            colors::yellow("note:")
        ),
    }

    // Install/patch every supported agent (idempotent, with backups).
    hook_install::install("all")?;

    // Show the resulting connection state so the user can confirm at a glance.
    println!("\n{}", colors::bold("Connection status"));
    integrations::status()?;
    Ok(())
}
