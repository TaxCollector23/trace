//! `trc integrations` and `trc integrations status` — list integration
//! surfaces and report what is live right now.

use anyhow::Result;
use trace_core::github;

use crate::colors;
use crate::daemon_ctl;

const INTEGRATIONS: &[(&str, &str, &str)] = &[
    ("Claude Code", "wrapper + hooks", "integrations/claude"),
    (
        "Codex",
        "lifecycle hooks + wrapper fallback",
        "integrations/codex",
    ),
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

pub fn status(json_output: bool) -> Result<()> {
    let daemon_port = daemon_ctl::running_port();
    let (token, src) = github::resolve_token();
    let connections = trace_core::integrations::detect_connections();

    if json_output {
        let agents = connections
            .iter()
            .map(|connection| {
                let definition = trace_core::integrations::by_id(connection.id);
                serde_json::json!({
                    "id": connection.id,
                    "name": connection.display_name,
                    "connected": connection.connected,
                    "how": connection.how,
                    "command_enforcement": definition.and_then(|d| d.command_enforcement),
                    "file_review": definition.and_then(|d| d.file_review),
                    "note": definition.map(|d| d.capability_note),
                })
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "daemon": daemon_port.map(|port| format!("http://127.0.0.1:{port}")),
                "github": { "token_available": token.is_some(), "source": src.as_str() },
                "agents": agents,
            }))?
        );
        return Ok(());
    }

    // Daemon
    match daemon_port {
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
    for connection in &connections {
        let mark = if connection.connected {
            any = true;
            colors::green("connected")
        } else {
            colors::dim("not connected")
        };
        println!(
            "  {:<13} {mark}  {}",
            connection.display_name,
            colors::dim(connection.how)
        );
    }
    if any {
        println!(
            "\n{}",
            colors::dim("Restart your agent after installing; Codex also needs /hooks trust before command blocking.")
        );
    } else {
        println!(
            "\nConnect every agent with `{}`.",
            colors::bold("trc integrations install all")
        );
    }
    Ok(())
}
