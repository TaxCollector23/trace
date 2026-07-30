//! `trg integrations` and `trg integrations status` — list integration surfaces
//! and report what is live right now.

use anyhow::Result;
use traceguard_core::github;

use crate::daemon_ctl;

const INTEGRATIONS: &[(&str, &str, &str)] = &[
    ("Claude Code", "wrapper + hooks", "integrations/claude"),
    ("Codex CLI", "wrapper", "integrations/codex"),
    ("Cursor", "MCP server", "integrations/cursor"),
    ("VS Code", "extension", "integrations/vscode"),
    (
        "Browser (ChatGPT/Claude/Gemini)",
        "extension",
        "integrations/browser-extension",
    ),
    (
        "GitHub",
        "Actions + App + direct repo read",
        "integrations/github",
    ),
];

pub fn list() -> Result<()> {
    println!("TraceGuard integrations:");
    for (name, kind, path) in INTEGRATIONS {
        println!("  • {name} — {kind}  ({path})");
    }
    println!("\nRun `trg integrations status` to check what is live now.");
    Ok(())
}

pub fn status() -> Result<()> {
    // Daemon
    match daemon_ctl::running_port() {
        Some(port) => println!("daemon:   running on http://127.0.0.1:{port}"),
        None => println!("daemon:   not running (start with `trg daemon start`)"),
    }

    // GitHub token (enables private repo reading + browser/MCP/CI flows)
    let (token, src) = github::resolve_token();
    match token {
        Some(_) => println!("github:   token available (source: {})", src.as_str()),
        None => println!("github:   no token (set GITHUB_TOKEN or run `gh auth login`)"),
    }

    // Browser extension reachability is the daemon itself.
    println!(
        "browser:  extension talks to the daemon; {}",
        if daemon_ctl::running_port().is_some() {
            "reachable"
        } else {
            "start the daemon first"
        }
    );

    println!("\nAdapters are in the integrations/ folder. See the docs for setup.");
    Ok(())
}
