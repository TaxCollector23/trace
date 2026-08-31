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
const CURSOR_HOOK_SH: &str = include_str!("../../../../integrations/cursor/cursor-hook.sh");
const OPENCODE_PLUGIN_JS: &str = include_str!("../../../../integrations/opencode/trace-plugin.js");

/// The list of installable agents. `install <agent>` picks one; `install
/// all` runs each in sequence and reports per-agent status.
pub const SUPPORTED: &[&str] = &["claude", "codex", "cursor", "windsurf", "opencode"];

/// Proper-noun display name for an agent key (the keys stay lowercase for the
/// CLI, but users see the brand's own capitalization).
pub fn display_name(agent: &str) -> &str {
    match agent {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "cursor" => "Cursor",
        "windsurf" => "Windsurf",
        "opencode" => "OpenCode",
        "vscode" => "VS Code",
        other => other,
    }
}

/// What an agent got wired up with, for the minimal install summary.
struct Wired {
    /// Short description of the mechanism, e.g. "MCP + enforcing guard hook".
    kind: &'static str,
    /// An optional one-line manual step (e.g. the Codex shell alias).
    note: Option<String>,
}

pub fn install(agent: &str) -> Result<()> {
    if agent == "all" {
        println!("{}", colors::bold("Connecting your agents to Trace"));
        let mut notes: Vec<String> = Vec::new();
        let mut any_err = false;
        for a in SUPPORTED {
            match install_one(a) {
                Ok(w) => {
                    println!(
                        "  {} {:<12} {}",
                        colors::green("✓"),
                        display_name(a),
                        colors::dim(w.kind)
                    );
                    if let Some(n) = w.note {
                        notes.push(n);
                    }
                }
                Err(e) => {
                    println!(
                        "  {} {:<12} {}",
                        colors::red("✗"),
                        display_name(a),
                        colors::dim(&e.to_string())
                    );
                    any_err = true;
                }
            }
        }
        print_next_steps(&notes);
        if any_err {
            anyhow::bail!("one or more installs failed");
        }
        return Ok(());
    }

    let w = install_one(agent)?;
    println!(
        "{} {} wired up ({}).",
        colors::green("✓"),
        display_name(agent),
        w.kind
    );
    print_next_steps(&w.note.into_iter().collect::<Vec<_>>());
    Ok(())
}

/// The short footer after an install: any manual steps, then the two reminders
/// that actually matter (restart the editor, run the daemon).
fn print_next_steps(notes: &[String]) {
    println!();
    if which_bin("node").is_none() {
        println!(
            "  {} Install Node 18+ for the MCP servers (Cursor, Windsurf, OpenCode).",
            colors::yellow("→")
        );
    }
    for n in notes {
        println!("  {} {}", colors::yellow("→"), n);
    }
    println!(
        "  {}",
        colors::dim("Restart any running editor to load the change.")
    );
    println!(
        "  {}",
        colors::dim("The guard needs the daemon running:  trc daemon start")
    );
}

fn install_one(agent: &str) -> Result<Wired> {
    match agent {
        "claude" => install_claude(),
        "codex" => install_codex(),
        "cursor" => install_cursor(),
        "windsurf" => install_windsurf(),
        "opencode" => install_opencode(),
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

/// Strip `//` line comments, `/* */` block comments, and trailing commas from
/// a JSONC / JSON5-ish document so it can be parsed by a strict JSON parser.
/// String literals (including escaped quotes) are preserved verbatim. This is
/// what lets us safely read a Cursor `mcp.json` or an `opencode.jsonc` that a
/// user has commented, instead of failing to parse and clobbering it.
fn strip_jsonc(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            out.push(c as char);
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                out.push('"');
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // line comment: skip to end of line
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                // block comment: skip to */
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            _ => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    // Remove trailing commas: a comma followed only by whitespace and then } or ].
    let cleaned = out;
    let mut result = String::with_capacity(cleaned.len());
    let rb = cleaned.as_bytes();
    let mut j = 0;
    while j < rb.len() {
        if rb[j] == b',' {
            let mut k = j + 1;
            while k < rb.len() && (rb[k] as char).is_whitespace() {
                k += 1;
            }
            if k < rb.len() && (rb[k] == b'}' || rb[k] == b']') {
                // drop the trailing comma
                j += 1;
                continue;
            }
        }
        result.push(rb[j] as char);
        j += 1;
    }
    result
}

/// Parse an existing config that may be JSONC. Tries strict JSON first, then a
/// comment/trailing-comma-stripped pass.
fn parse_config(raw: &str) -> Result<Value> {
    if raw.trim().is_empty() {
        return Ok(Value::Object(Default::default()));
    }
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        return Ok(v);
    }
    let stripped = strip_jsonc(raw);
    serde_json::from_str::<Value>(&stripped)
        .context("existing config is not valid JSON (even after stripping comments)")
}

/// Merge (rather than overwrite) a JSON settings file. Backs up on first
/// modification, so a user can always undo. Deep-merges maps; arrays get
/// deduped by structural equality.
///
/// Safety: if the existing file cannot be parsed even as JSONC, this ABORTS
/// with an error and leaves the file untouched. It never falls back to an
/// empty object, which would silently overwrite the user's whole config.
fn merge_json_file(path: &Path, patch: &Value) -> Result<bool> {
    let existing = if path.exists() {
        let raw =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        parse_config(&raw)
            .with_context(|| format!("{}: refusing to overwrite it", path.display()))?
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

/// Set our own named entry inside a top-level container object of `config_path`,
/// REPLACING any previous value for that entry while preserving every sibling.
///
/// This is the right merge for an MCP server entry: we own the whole "trace"
/// object, so deep-merging it would (for example) append to its `args` array on
/// every re-install. JSONC-safe and abort-on-unparseable, like `merge_json_file`.
fn set_mcp_entry(
    config_path: &Path,
    container: &str,
    entry_key: &str,
    entry: Value,
    schema: Option<&str>,
) -> Result<bool> {
    let mut config = if config_path.exists() {
        let raw = fs::read_to_string(config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        parse_config(&raw)
            .with_context(|| format!("{}: refusing to overwrite it", config_path.display()))?
    } else {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        Value::Object(Default::default())
    };
    if !config.is_object() {
        anyhow::bail!("{} is not a JSON object", config_path.display());
    }
    let before = config.clone();
    // Safe: the `!config.is_object()` bail above guarantees an object here.
    let obj = config.as_object_mut().unwrap();
    if let Some(s) = schema {
        obj.entry("$schema".to_string())
            .or_insert_with(|| Value::String(s.to_string()));
    }
    let cont = obj
        .entry(container.to_string())
        .or_insert_with(|| Value::Object(Default::default()));
    if !cont.is_object() {
        *cont = Value::Object(Default::default());
    }
    // Safe: `cont` was just reset to an object on the line above if it wasn't one.
    cont.as_object_mut()
        .unwrap()
        .insert(entry_key.to_string(), entry);

    if config == before {
        return Ok(false);
    }
    if config_path.exists() {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let backup = config_path.with_extension(format!("trace-backup-{ts}"));
        fs::copy(config_path, &backup).ok();
    }
    fs::write(config_path, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", config_path.display()))?;
    Ok(true)
}

fn install_claude() -> Result<Wired> {
    let hook = trace_integrations_dir()?
        .join("claude")
        .join("trace-hook.sh");
    write_executable(&hook, CLAUDE_HOOK_SH)?;

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
    merge_json_file(&settings, &patch)?;
    Ok(Wired {
        kind: "PreToolUse + PostToolUse hooks (enforced)",
        note: None,
    })
}

fn install_codex() -> Result<Wired> {
    let adapter = trace_integrations_dir()?
        .join("codex")
        .join("codex-adapter.sh");
    write_executable(&adapter, CODEX_ADAPTER_SH)?;
    Ok(Wired {
        kind: "wrapper script",
        note: Some(format!(
            "Codex: add to your shell rc  ->  alias codex=\"{}\"",
            adapter.display()
        )),
    })
}

fn install_cursor() -> Result<Wired> {
    let home = dirs::home_dir().context("no home directory")?;
    let cursor_dir = home.join(".cursor");

    // 1. MCP server: Trace's read tools (recent runs, patch summary, rollback).
    write_mcp("cursor", &cursor_dir.join("mcp.json"))?;

    // 2. Enforcing guard: a beforeShellExecution hook that denies dangerous
    //    commands before Cursor runs them.
    let hook = trace_integrations_dir()?
        .join("cursor")
        .join("cursor-hook.sh");
    write_executable(&hook, CURSOR_HOOK_SH)?;
    let hooks_json = cursor_dir.join("hooks.json");
    let patch = json!({
        "version": 1,
        "hooks": {
            "beforeShellExecution": [{ "command": hook.display().to_string() }]
        }
    });
    merge_json_file(&hooks_json, &patch)?;
    Ok(Wired {
        kind: "MCP tools + enforcing guard hook",
        note: None,
    })
}

fn install_windsurf() -> Result<Wired> {
    // Windsurf ships MCP config at ~/.codeium/windsurf/mcp_config.json, the
    // same `mcpServers` shape as Cursor, in its own server directory. Windsurf
    // does not expose a command hook today, so this is MCP (read tools) only.
    let config = dirs::home_dir()
        .context("no home directory")?
        .join(".codeium")
        .join("windsurf")
        .join("mcp_config.json");
    write_mcp("windsurf", &config)?;
    Ok(Wired {
        kind: "MCP tools (read only)",
        note: None,
    })
}

/// opencode (github.com/sst/opencode) is an open-source terminal agent that
/// speaks MCP. Unlike Cursor/Windsurf (which use a `mcpServers` map with
/// `command` + `args`), opencode uses an `mcp` map whose entries are
/// `{ type: "local", command: [ ... ], enabled: true }`, read from the global
/// config at `~/.config/opencode/opencode.json` (XDG-aware). We reuse the same
/// daemon-backed MCP server the other editors use.
fn install_opencode() -> Result<Wired> {
    let server = trace_integrations_dir()?.join("opencode").join("index.js");
    write_executable(&server, CURSOR_MCP_JS)?;

    let home = dirs::home_dir().context("no home directory")?;
    let config_base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let config_path = config_base.join("opencode").join("opencode.json");

    // Replace (not deep-merge) our own `mcp.trace` entry, and set $schema only
    // if the file has none.
    let entry = json!({
        "type": "local",
        "command": ["node", server.display().to_string()],
        "enabled": true
    });
    set_mcp_entry(
        &config_path,
        "mcp",
        "trace",
        entry,
        Some("https://opencode.ai/config.json"),
    )?;

    // Enforcing guard: a `tool.execute.before` plugin, auto-loaded from the
    // global plugin dir, that blocks dangerous bash commands before they run.
    let plugin = config_base.join("opencode").join("plugin").join("trace.js");
    write_executable(&plugin, OPENCODE_PLUGIN_JS)?;

    Ok(Wired {
        kind: "MCP tools + enforcing guard plugin",
        note: None,
    })
}

/// Write the shared daemon-backed MCP server into `~/.trace/integrations/
/// <server_dir>/index.js` and register it as an `mcpServers.trace` entry in
/// `config_path` (Cursor/Windsurf shape). Quiet; callers report the summary.
fn write_mcp(server_dir: &str, config_path: &Path) -> Result<()> {
    let server = trace_integrations_dir()?.join(server_dir).join("index.js");
    write_executable(&server, CURSOR_MCP_JS)?;
    let entry = json!({ "command": "node", "args": [server.display().to_string()] });
    set_mcp_entry(config_path, "mcpServers", "trace", entry, None)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_jsonc_removes_comments_and_trailing_commas() {
        let src = r#"{
            // a line comment
            "a": 1, /* block */ "b": [1, 2,],
        }"#;
        let v: Value = serde_json::from_str(&strip_jsonc(src)).unwrap();
        assert_eq!(v["a"], 1);
        assert_eq!(v["b"], serde_json::json!([1, 2]));
    }

    #[test]
    fn strip_jsonc_preserves_comment_like_text_inside_strings() {
        let src = r#"{ "url": "https://x.y/z", "note": "a // b /* c */" }"#;
        let v: Value = serde_json::from_str(&strip_jsonc(src)).unwrap();
        assert_eq!(v["url"], "https://x.y/z");
        assert_eq!(v["note"], "a // b /* c */");
    }

    #[test]
    fn merge_preserves_a_jsonc_config_and_adds_our_entry() {
        let dir = std::env::temp_dir().join(format!("trc-jsonc-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mcp.json");
        // A user's config written as JSONC (comment + trailing comma).
        fs::write(
            &path,
            "{\n  // my servers, do not touch\n  \"mcpServers\": {\n    \"mine\": { \"command\": \"node\" },\n  }\n}\n",
        )
        .unwrap();

        let patch = json!({ "mcpServers": { "trace": { "command": "node", "args": ["i.js"] } } });
        let changed = merge_json_file(&path, &patch).unwrap();
        assert!(changed);

        let out: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        // The user's server survived, and ours was added.
        assert_eq!(out["mcpServers"]["mine"]["command"], "node");
        assert_eq!(out["mcpServers"]["trace"]["command"], "node");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn set_mcp_entry_replaces_our_entry_and_keeps_siblings() {
        let dir = std::env::temp_dir().join(format!("trc-setmcp-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mcp.json");
        // A config where our own trace entry has a STALE path, plus a user's server.
        fs::write(
            &path,
            r#"{ "mcpServers": {
                "mine": { "command": "node", "args": ["keep.js"] },
                "trace": { "command": "node", "args": ["OLD/path.js"] }
            } }"#,
        )
        .unwrap();

        let entry = json!({ "command": "node", "args": ["NEW/path.js"] });
        let changed = set_mcp_entry(&path, "mcpServers", "trace", entry, None).unwrap();
        assert!(changed);

        let out: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        // Our args were REPLACED (one path, the new one), not appended.
        assert_eq!(
            out["mcpServers"]["trace"]["args"],
            serde_json::json!(["NEW/path.js"])
        );
        // The user's server is untouched.
        assert_eq!(
            out["mcpServers"]["mine"]["args"],
            serde_json::json!(["keep.js"])
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn merge_refuses_to_clobber_an_unparseable_file() {
        let dir = std::env::temp_dir().join(format!("trc-bad-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cfg.json");
        let garbage = "this is not json at all { ] ohno";
        fs::write(&path, garbage).unwrap();

        let patch = json!({ "x": 1 });
        let result = merge_json_file(&path, &patch);
        assert!(result.is_err(), "must error, not overwrite");
        // The file is untouched.
        assert_eq!(fs::read_to_string(&path).unwrap(), garbage);
        fs::remove_dir_all(&dir).ok();
    }
}
