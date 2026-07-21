//! `trace doctor` — system checks with actionable fixes.

use anyhow::Result;
use trace_core::{agents, paths};

use crate::colors;
use crate::daemon_ctl;

fn tool_version(bin: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(bin).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let combined = if s.trim().is_empty() {
        String::from_utf8_lossy(&out.stderr).to_string()
    } else {
        s.to_string()
    };
    combined.lines().next().map(|l| l.trim().to_string())
}

fn line(label: &str, ok: bool, detail: &str) {
    let mark = if ok {
        colors::green("✓")
    } else {
        colors::red("✗")
    };
    let detail = if ok {
        colors::dim(detail)
    } else {
        detail.to_string()
    };
    println!("  {mark} {label:<16} {detail}");
}

fn heading(s: &str) {
    println!("{}", colors::bold(s));
}

pub fn run() -> Result<()> {
    println!(
        "{}\n",
        colors::bold(&format!("Trace doctor — v{}", trace_core::VERSION))
    );

    // Core toolchain.
    heading("Toolchain:");
    match tool_version("git", &["--version"]) {
        Some(v) => line("git", true, &v),
        None => line(
            "git",
            false,
            "not found — install Git (rollback/scan need it)",
        ),
    }
    match tool_version("node", &["--version"]) {
        Some(v) => line("node", true, &v),
        None => line(
            "node",
            false,
            "not found — install Node.js for npm-based agents",
        ),
    }
    match tool_version("npm", &["--version"]) {
        Some(v) => line("npm", true, &format!("v{v}")),
        None => line("npm", false, "not found"),
    }

    // Clipboard.
    heading("\nClipboard:");
    match agents::copy_to_clipboard("") {
        Ok(t) => line("clipboard", true, &format!("available ({t})")),
        Err(e) => line("clipboard", false, &e),
    }

    // Daemon.
    heading("\nDaemon:");
    match daemon_ctl::running_port() {
        Some(port) => line(
            "daemon",
            true,
            &format!("running on http://127.0.0.1:{port}"),
        ),
        None => line(
            "daemon",
            false,
            "not running — start with `trace daemon start`",
        ),
    }

    // Paths.
    heading("\nPaths:");
    if let Ok(db) = paths::database_path() {
        line("database", db.exists(), &db.display().to_string());
    }
    let bin_dir = paths::bin_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    let on_path = std::env::var("PATH").unwrap_or_default().contains(&bin_dir);
    line(
        "bin on PATH",
        on_path,
        &if on_path {
            bin_dir.clone()
        } else {
            format!("add to PATH: {bin_dir}")
        },
    );

    // Installed AI tools.
    heading("\nAI tools detected:");
    let all = agents::detect_all();
    let installed: Vec<_> = all.iter().filter(|a| a.installed).collect();
    let web: Vec<_> = all.iter().filter(|a| a.surface == "web").collect();
    if installed.is_empty() {
        line(
            "cli agents",
            false,
            "none found — web tools still work (chatgpt, gemini, …)",
        );
    } else {
        line(
            "cli agents",
            true,
            &installed
                .iter()
                .map(|a| a.id.clone())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    line(
        "web agents",
        true,
        &format!("{} available (no install needed)", web.len()),
    );

    println!("\nRun a detected tool directly, e.g. `trace run claude \"<prompt>\"`.");
    Ok(())
}
