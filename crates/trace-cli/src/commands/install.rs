//! `trc install <target>` — the friendly, top-level installer.
//!
//! Today the only target is `agents`, which wires Trace into every supported
//! coding agent (Claude Code, Cursor, Windsurf, Codex) and starts the local
//! daemon so live review works immediately. It's a thin, well-labeled wrapper
//! over the same idempotent installer behind `trc integrations install all`,
//! surfaced as a primary onboarding command.

use anyhow::Result;

use crate::commands::hook_install;

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
    // Install/patch every supported agent and start the daemon. The same
    // implementation backs `trc integrations install all`.
    hook_install::install("all")?;
    Ok(())
}
