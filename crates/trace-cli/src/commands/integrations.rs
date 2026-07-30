//! `trace integrations` and `trace integrations status` — list integration
//! surfaces and report what is live right now.

use anyhow::Result;
use trace_core::github;

use crate::colors;
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
    println!("{}", colors::bold("Trace integrations:"));
    for (name, kind, path) in INTEGRATIONS {
        println!("  • {name} — {kind}  ({})", colors::dim(path));
    }
    println!("\nRun `trace integrations status` to check what is live now.");
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
            "daemon:   {} (start with `trace daemon start`)",
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

    println!("\nAdapters are in the integrations/ folder. See the docs for setup.");
    Ok(())
}
