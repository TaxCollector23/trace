//! `trc doctor` — system checks with actionable fixes.

use anyhow::Result;
use trace_core::{agents, connections, importers::ImportContext, paths, Store};

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
    let version_details = trace_core::version_details();
    println!(
        "{}\n",
        colors::bold(&format!("Trace doctor — {version_details}"))
    );

    // Version + rule-pack provenance, so `doctor` and `--version` agree and a
    // user can see exactly which rules are active.
    heading("Version:");
    line("trace", true, &version_details);
    line(
        "rule pack",
        true,
        &format!(
            "{} ({} rules)",
            trace_core::rule_pack_version(),
            trace_core::rule_count()
        ),
    );
    line("pricing", true, &trace_core::cost::pricing_version());
    println!();

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
            "not running — start with `trc daemon start`",
        ),
    }

    // Policy engine self-check.
    heading("\nPolicy engine:");
    let report = trace_core::run_policy_eval();
    line(
        "self-check",
        report.passed == report.total,
        &format!(
            "{}/{} fixtures passed ({} false positives, {} missed; {:.0}% precision, {:.0}% recall, {:.0}% F1) — full report: `trc self-check`",
            report.passed,
            report.total,
            report.false_positives,
            report.false_negatives,
            report.precision * 100.0,
            report.recall * 100.0,
            report.f1 * 100.0
        ),
    );

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

    // Connected agent integrations (same detection `trc integrations status`
    // uses), so `doctor` is a single one-stop health check.
    heading("\nAgent integrations:");
    let connections = trace_core::integrations::detect_connections();
    let any_connected = connections.iter().any(|c| c.connected);
    for c in &connections {
        line(
            c.display_name,
            c.connected,
            if c.connected { c.how } else { "not connected" },
        );
    }
    if !any_connected {
        println!(
            "  {}",
            colors::dim("wire one up with `trc integrations install <agent>` (or `all`)")
        );
    }

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

    println!("\nRun a detected tool directly, e.g. `trc run claude \"<prompt>\"`.");
    Ok(())
}

/// `trc doctor agents` — detailed, source-aware connection diagnostics.
pub fn agents(json: bool) -> Result<()> {
    let store = Store::open(&paths::database_path()?)?;
    let context = ImportContext::from_environment()?;
    let rows = connections::inspect(&store, &context)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "agents": rows }))?
        );
        return Ok(());
    }

    println!("{}", colors::bold("Trace doctor — agents"));
    for row in rows {
        let installed = if row.installed {
            "installed"
        } else {
            "not installed"
        };
        let connected = if row.connected {
            "connected"
        } else {
            row.status.as_str()
        };
        println!(
            "\n{}  {} · {}",
            colors::bold(&row.name),
            installed,
            connected
        );
        if let Some(path) = row.path {
            println!("  executable: {}", colors::dim(&path));
        }
        if let Some(version) = row.version {
            println!("  version: {}", colors::dim(&version));
        }
        println!("  session data: {}", row.session_paths.join(", "));
        println!(
            "  found: {} · imported: {} · parse errors: {} · last event: {}",
            row.session_data_found,
            row.events_imported,
            row.parse_errors,
            row.last_event_at.as_deref().unwrap_or("unavailable")
        );
        println!("  action: {}", row.action);
        println!(
            "  capabilities: tokens={} cost={} subagents={} tool_calls={} command_guard={}",
            row.capabilities.tokens.as_str(),
            row.capabilities.cost.as_str(),
            row.capabilities.subagents.as_str(),
            row.capabilities.tool_calls.as_str(),
            row.capabilities.command_guard.as_str(),
        );
    }
    Ok(())
}
