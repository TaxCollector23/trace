//! `trc integrations` and `trc integrations status` — list integration
//! surfaces and report what is live right now.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use trace_core::github;

use crate::colors;
use crate::daemon_ctl;

/// Detect which agents are actually wired up right now by looking for Trace's
/// entry in each agent's own config file (the same files the installer patches
/// in `hook_install.rs`). Returns (agent, connected?, how).
fn agent_connections() -> Vec<(&'static str, bool, &'static str)> {
    let home = dirs::home_dir();
    let contains = |rel: &[&str], needle: &str| -> bool {
        let Some(h) = home.as_ref() else { return false };
        let mut p: PathBuf = h.clone();
        for seg in rel {
            p.push(seg);
        }
        fs::read_to_string(&p)
            .map(|s| s.contains(needle))
            .unwrap_or(false)
    };
    let exists = |rel: &[&str]| -> bool {
        let Some(h) = home.as_ref() else { return false };
        let mut p: PathBuf = h.clone();
        for seg in rel {
            p.push(seg);
        }
        p.exists()
    };
    vec![
        (
            "Claude Code",
            contains(&[".claude", "settings.json"], "trace-hook"),
            "PreToolUse + PostToolUse hooks",
        ),
        (
            "Cursor",
            contains(&[".cursor", "mcp.json"], ".trace/integrations/cursor"),
            "MCP server",
        ),
        (
            "Windsurf",
            contains(
                &[".codeium", "windsurf", "mcp_config.json"],
                ".trace/integrations/cursor",
            ),
            "MCP server",
        ),
        (
            "Codex CLI",
            exists(&[".trace", "integrations", "codex", "codex-adapter.sh"]),
            "wrapper script (add the shell alias to finish)",
        ),
        (
            "opencode",
            contains(
                &[".config", "opencode", "opencode.json"],
                ".trace/integrations/opencode",
            ),
            "MCP server",
        ),
    ]
}

const INTEGRATIONS: &[(&str, &str, &str)] = &[
    ("Claude Code", "wrapper + hooks", "integrations/claude"),
    ("Codex CLI", "wrapper", "integrations/codex"),
    ("Cursor", "MCP server", "integrations/cursor"),
    ("opencode", "MCP server", "integrations/opencode"),
    ("VS Code", "extension", "integrations/vscode"),
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
