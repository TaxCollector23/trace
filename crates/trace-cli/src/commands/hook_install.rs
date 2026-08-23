//! `trc integrations install <agent>` — one-shot hook/MCP installer.
//!
//! Ships every integration's source file embedded in the binary so the
//! install works from a fresh binary with no repo checkout. For each agent:
//!
//!   1. Writes the wrapper/hook script to `~/.trace/integrations/<agent>/`
//!   2. Patches the agent's own config file (Claude settings, Cursor MCP,
//!      Windsurf MCP) idempotently — merges rather than overwrites
//!   3. Prints what actually changed and any remaining manual step
//!
//! Every step is idempotent. Re-running the installer for the same agent
//! is a no-op that re-verifies the install is intact.
//!
//! Safety: never touches an existing settings file without first backing it
//! up to `<file>.trace-backup-<timestamp>`, and never removes an existing
//! hook the user has already configured — only appends Trace's own entry.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::colors;

// Embedded integration sources. Kept in sync with `integrations/` at build
// time via include_str! — a new file there just needs to be added below.
const CLAUDE_HOOK_SH: &str = include_str!("../../../../integrations/claude/trace-hook.sh");
const CODEX_ADAPTER_SH: &str = include_str!("../../../../integrations/codex/codex-adapter.sh");
const CURSOR_MCP_JS: &str = include_str!("../../../../integrations/cursor/src/index.js");

/// The list of installable agents. `install <agent>` picks one; `install
/// all` runs each in sequence and reports per-agent status.
pub const SUPPORTED: &[&str] = &["claude", "codex", "cursor", "windsurf", "vscode"];

pub fn install(agent: &str) -> Result<()> {
    if agent == "all" {
        let mut any_err = false;
        for a in SUPPORTED {
            println!("\n{}", colors::bold(&format!("─── {a} ───")));
            if let Err(e) = install_one(a) {
                eprintln!("  {} {e}", colors::red("failed:"));
                any_err = true;
            }
        }
        if any_err {
            anyhow::bail!("one or more installs failed");
        }
        return Ok(());
    }
    install_one(agent)
}

fn install_one(agent: &str) -> Result<()> {
    match agent {
        "claude" => install_claude(),
        "codex" => install_codex(),
        "cursor" => install_cursor(),
        "windsurf" => install_windsurf(),
        "vscode" => install_vscode(),
        other => anyhow::bail!(
            "unknown agent '{other}'. Supported: {}",
            SUPPORTED.join(", ")
        ),
    }
}

fn trace_integrations_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home directory")?;
    Ok(home.join(".trace").join("integrations"))
}

fn write_executable(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// Merge (rather than overwrite) a JSON settings file. Backs up on first
/// modification, so a user can always undo. Deep-merges maps; arrays get
/// deduped by structural equality.
fn merge_json_file(path: &Path, patch: &Value) -> Result<bool> {
    let existing = if path.exists() {
        let raw =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&raw).unwrap_or(Value::Object(Default::default()))
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        Value::Object(Default::default())
    };

    let mut merged = existing.clone();
    deep_merge(&mut merged, patch);

    if merged == existing {
        return Ok(false);
    }

    if path.exists() {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let backup = path.with_extension(format!("trace-backup-{ts}"));
        fs::copy(path, &backup)
            .with_context(|| format!("backing up {} -> {}", path.display(), backup.display()))?;
    }

    fs::write(path, serde_json::to_string_pretty(&merged)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

fn deep_merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                deep_merge(t.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (Value::Array(t), Value::Array(p)) => {
            for v in p {
                if !t.iter().any(|existing| existing == v) {
                    t.push(v.clone());
                }
            }
        }
        (t, p) => {
            *t = p.clone();
        }
    }
}

fn install_claude() -> Result<()> {
    let dir = trace_integrations_dir()?.join("claude");
    let hook = dir.join("trace-hook.sh");
    write_executable(&hook, CLAUDE_HOOK_SH)?;
    println!("  wrote {}", colors::dim(&hook.display().to_string()));

    let home = dirs::home_dir().context("no home directory")?;
    let settings = home.join(".claude").join("settings.json");
    let hook_cmd = hook.display().to_string();
    let patch = json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [{ "type": "command", "command": hook_cmd }]
            }],
            "PostToolUse": [{
                "matcher": "Edit|Write|MultiEdit|NotebookEdit",
                "hooks": [{ "type": "command", "command": hook_cmd }]
            }]
        }
    });
    let changed = merge_json_file(&settings, &patch)?;
    if changed {
        println!(
            "  {} {}",
            colors::green("patched"),
            colors::dim(&settings.display().to_string())
        );
    } else {
        println!(
            "  {} {}",
            colors::dim("already installed:"),
            colors::dim(&settings.display().to_string())
        );
    }
    println!(
        "\n  {} Claude Code will call the Trace hook on Bash + Edit/Write tool use.",
        colors::green("✓")
    );
    println!("  Start the daemon with `trc daemon start` to enable live review.");
    Ok(())
}

fn install_codex() -> Result<()> {
    let dir = trace_integrations_dir()?.join("codex");
    let adapter = dir.join("codex-adapter.sh");
    write_executable(&adapter, CODEX_ADAPTER_SH)?;
    println!("  wrote {}", colors::dim(&adapter.display().to_string()));
    println!(
        "\n  {} Codex integration is a wrapper script (no upstream config to patch).",
        colors::green("✓")
    );
    println!(
        "  To route every `codex` call through Trace, add this to your shell rc:\n    {} \"{}\"",
        colors::bold("alias codex="),
        adapter.display()
    );
    Ok(())
}

fn install_cursor() -> Result<()> {
    install_mcp(
        "cursor",
        dirs::home_dir()
            .context("no home directory")?
            .join(".cursor")
            .join("mcp.json"),
    )
}

fn install_windsurf() -> Result<()> {
    // Windsurf ships MCP config at ~/.codeium/windsurf/mcp_config.json.
    // Same shape as Cursor's mcp.json (mcpServers map), so the same MCP
    // server binary works — no per-editor fork needed.
    install_mcp(
        "windsurf",
        dirs::home_dir()
            .context("no home directory")?
            .join(".codeium")
            .join("windsurf")
            .join("mcp_config.json"),
    )
}

fn install_mcp(agent: &str, config_path: PathBuf) -> Result<()> {
    let dir = trace_integrations_dir()?.join("cursor"); // shared MCP source
    let server = dir.join("index.js");
    write_executable(&server, CURSOR_MCP_JS)?;
    println!("  wrote {}", colors::dim(&server.display().to_string()));

    // Preflight: MCP requires Node.js. Warn but don't fail — user may
    // install Node after.
    if which_bin("node").is_none() {
        println!(
            "  {} Node.js not found on PATH — install Node ≥ 18 for the MCP server to run.",
            colors::yellow("warning:")
        );
    }

    let patch = json!({
        "mcpServers": {
            "trace": {
                "command": "node",
                "args": [server.display().to_string()]
            }
        }
    });
    let changed = merge_json_file(&config_path, &patch)?;
    if changed {
        println!(
            "  {} {}",
            colors::green("patched"),
            colors::dim(&config_path.display().to_string())
        );
    } else {
        println!(
            "  {} {}",
            colors::dim("already installed:"),
            colors::dim(&config_path.display().to_string())
        );
    }
    println!(
        "\n  {} {} will see Trace's MCP tools after a restart.",
        colors::green("✓"),
        agent
    );
    Ok(())
}

fn install_vscode() -> Result<()> {
    // The VS Code extension isn't installable from here — it's an
    // extension the user installs from the marketplace or a .vsix path.
    // We can print the exact command they need.
    println!("  VS Code extension isn't scriptable-installable from here (no marketplace ID yet).");
    println!(
        "  Once published, install with:  {}",
        colors::bold("code --install-extension trace")
    );
    println!(
        "  Meanwhile, load unpacked from  {}",
        colors::dim("integrations/vscode/")
    );
    Ok(())
}

fn which_bin(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let cand = dir.join(name);
            if cand.is_file() {
                Some(cand)
            } else {
                None
            }
        })
    })
}
