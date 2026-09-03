//! `trc integrations` and `trc integrations status` — list integration
//! surfaces and report what is live right now.

use anyhow::Result;
use trace_core::github;

use crate::colors;
use crate::daemon_ctl;

/// Which agents are actually wired up right now. Detection itself lives in
/// `trace_core::integrations` (reads the same config files the installer
/// patches in `hook_install.rs`) so the CLI and the daemon's
/// `/api/integrations/coverage` route can never disagree about what
/// "connected" means. Returns (display_name, connected?, how).
fn agent_connections() -> Vec<(&'static str, bool, &'static str)> {
    trace_core::integrations::detect_connections()
        .into_iter()
        .map(|c| (c.display_name, c.connected, c.how))
        .collect()
}

const INTEGRATIONS: &[(&str, &str, &str)] = &[
    ("Claude Code", "wrapper + hooks", "integrations/claude"),
    ("Codex CLI", "wrapper", "integrations/codex"),
    ("Cursor", "MCP tools + guard hook", "integrations/cursor"),
    ("Windsurf", "MCP server", "integrations/windsurf"),
    (
        "OpenCode",
        "MCP tools + guard plugin",
        "integrations/opencode",
    ),
    (
        "GitHub",
        "Actions + App + direct repo read",
        "integrations/github",
    ),
];

pub fn list() -> Result<()> {
    println!("{}", colors::bold("Trace integrations:"));
    for (name, kind, path) in INTEGRATIONS {
        println!("  • {name} — {kind}  ({})", colors::dim(path));
    }
    println!("\nRun `trc integrations status` to check what is live now.");
    Ok(())
}

pub fn status() -> Result<()> {
    // Daemon
    match daemon_ctl::running_port() {
        Some(port) => println!(
            "daemon:   {} on http://127.0.0.1:{port}",
            colors::green("running")
        ),
        None => println!(
            "daemon:   {} (start with `trc daemon start`)",
            colors::red("not running")
        ),
    }

    // GitHub token (enables private repo reading + MCP/CI flows)
    let (token, src) = github::resolve_token();
    match token {
        Some(_) => println!(
            "github:   {} (source: {})",
            colors::green("token available"),
            src.as_str()
        ),
        None => println!(
            "github:   {} (set GITHUB_TOKEN or run `gh auth login`)",
            colors::dim("no token")
        ),
    }

    // Which agents are connected right now (detected from their config files).
    println!("\n{}", colors::bold("Connected agents:"));
    let mut any = false;
    for (name, connected, how) in agent_connections() {
        let mark = if connected {
            any = true;
            colors::green("connected")
        } else {
            colors::dim("not connected")
        };
        println!("  {name:<13} {mark}  {}", colors::dim(how));
    }
    if any {
        println!(
            "\n{}",
            colors::dim("Restart Cursor/Windsurf to load the MCP server; Claude hooks apply to new sessions.")
        );
    } else {
        println!(
            "\nConnect every agent with `{}`.",
            colors::bold("trc integrations install all")
        );
    }
    Ok(())
}
