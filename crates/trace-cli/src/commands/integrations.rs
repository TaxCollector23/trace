//! `trace integrations` and `trace integrations status` — list integration
//! surfaces and report what is live right now.

use anyhow::Result;
use trace_core::github;

use crate::daemon_ctl;

const INTEGRATIONS: &[(&str, &str, &str)] = &[
    ("Claude Code", "wrapper + hooks", "integrations/claude"),
    ("Codex CLI", "wrapper", "integrations/codex"),
    ("Cursor", "MCP server", "integrations/cursor"),
    ("VS Code", "extension", "integrations/vscode"),
    (
        "GitHub",
        "Actions + App + direct repo read",
        "integrations/github",
    ),
];

pub fn list() -> Result<()> {
    println!("Trace integrations:");
    for (name, kind, path) in INTEGRATIONS {
        println!("  • {name} — {kind}  ({path})");
    }
    println!("\nRun `trace integrations status` to check what is live now.");
    Ok(())
}

pub fn status() -> Result<()> {
    // Daemon
    match daemon_ctl::running_port() {
        Some(port) => println!("daemon:   running on http://127.0.0.1:{port}"),
        None => println!("daemon:   not running (start with `trace daemon start`)"),
    }

    // GitHub token (enables private repo reading + MCP/CI flows)
    let (token, src) = github::resolve_token();
    match token {
        Some(_) => println!("github:   token available (source: {})", src.as_str()),
        None => println!("github:   no token (set GITHUB_TOKEN or run `gh auth login`)"),
    }

    println!("\nAdapters are in the integrations/ folder. See the docs for setup.");
    Ok(())
}
