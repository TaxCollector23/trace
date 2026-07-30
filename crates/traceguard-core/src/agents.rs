//! AI tool registry, detection, and launch planning.
//!
//! TraceGuard can launch a cleaned/hardened prompt into many AI tools. For CLI
//! tools it detects whether the binary is installed and runs it. For web tools
//! it copies the prompt to the clipboard and opens the site — it never pretends
//! it injected text into a web page when it did not.

use serde::{Deserialize, Serialize};

/// How a tool is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// A local command-line tool (detect binary, run it).
    Cli,
    /// A web app (copy prompt to clipboard, open the URL).
    Web,
    /// Available both ways; prefer CLI when installed, else web.
    Both,
}

/// A supported AI tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Stable id used on the CLI, e.g. "claude", "chatgpt".
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    pub surface: Surface,
    /// Binary name to detect on PATH (for CLI/Both).
    pub bin: Option<&'static str>,
    /// Web URL (for Web/Both).
    pub url: Option<&'static str>,
    /// Install hint shown when the binary is missing.
    pub install_hint: Option<&'static str>,
    /// Rough category used by the auto-router.
    pub category: Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Coding,
    Research,
    Marketing,
    Local,
    Github,
    Generic,
}

/// The full registry of supported tools.
pub fn registry() -> Vec<Agent> {
    use Category::*;
    use Surface::*;
    vec![
        // Coding agents (CLI-first)
        Agent {
            id: "claude",
            name: "Claude Code",
            surface: Both,
            bin: Some("claude"),
            url: Some("https://claude.ai"),
            install_hint: Some("npm install -g @anthropic-ai/claude-code"),
            category: Coding,
        },
        Agent {
            id: "codex",
            name: "Codex CLI",
            surface: Cli,
            bin: Some("codex"),
            url: None,
            install_hint: Some("npm install -g @openai/codex"),
            category: Coding,
        },
        Agent {
            id: "cursor",
            name: "Cursor",
            surface: Cli,
            bin: Some("cursor"),
            url: Some("https://cursor.com"),
            install_hint: Some("Install Cursor from https://cursor.com (enables the `cursor` CLI)"),
            category: Coding,
        },
        Agent {
            id: "windsurf",
            name: "Windsurf",
            surface: Cli,
            bin: Some("windsurf"),
            url: Some("https://windsurf.com"),
            install_hint: Some("Install Windsurf from https://windsurf.com"),
            category: Coding,
        },
        Agent {
            id: "aider",
            name: "Aider",
            surface: Cli,
            bin: Some("aider"),
            url: None,
            install_hint: Some("python -m pip install aider-install && aider-install"),
            category: Coding,
        },
        Agent {
            id: "opencode",
            name: "OpenCode",
            surface: Cli,
            bin: Some("opencode"),
            url: None,
            install_hint: Some("npm install -g opencode-ai"),
            category: Coding,
        },
        Agent {
            id: "continue",
            name: "Continue",
            surface: Cli,
            bin: Some("cn"),
            url: Some("https://continue.dev"),
            install_hint: Some("npm install -g @continuedev/cli"),
            category: Coding,
        },
        Agent {
            id: "copilot",
            name: "GitHub Copilot CLI",
            surface: Cli,
            bin: Some("copilot"),
            url: None,
            install_hint: Some("npm install -g @github/copilot"),
            category: Github,
        },
        Agent {
            id: "gh",
            name: "GitHub CLI",
            surface: Cli,
            bin: Some("gh"),
            url: None,
            install_hint: Some("brew install gh  (or https://cli.github.com)"),
            category: Github,
        },
        // Research / chat (web-first)
        Agent {
            id: "chatgpt",
            name: "ChatGPT",
            surface: Web,
            bin: None,
            url: Some("https://chatgpt.com"),
            install_hint: None,
            category: Research,
        },
        Agent {
            id: "claude-web",
            name: "Claude (web)",
            surface: Web,
            bin: None,
            url: Some("https://claude.ai/new"),
            install_hint: None,
            category: Research,
        },
        Agent {
            id: "gemini",
            name: "Gemini",
            surface: Both,
            bin: Some("gemini"),
            url: Some("https://gemini.google.com/app"),
            install_hint: Some("npm install -g @google/gemini-cli"),
            category: Research,
        },
        Agent {
            id: "perplexity",
            name: "Perplexity",
            surface: Web,
            bin: None,
            url: Some("https://www.perplexity.ai"),
            install_hint: None,
            category: Research,
        },
        // Consoles
        Agent {
            id: "groq",
            name: "Groq Console",
            surface: Web,
            bin: None,
            url: Some("https://console.groq.com"),
            install_hint: None,
            category: Research,
        },
        Agent {
            id: "openrouter",
            name: "OpenRouter",
            surface: Web,
            bin: None,
            url: Some("https://openrouter.ai/chat"),
            install_hint: None,
            category: Research,
        },
        // Local model runtimes
        Agent {
            id: "ollama",
            name: "Ollama",
            surface: Cli,
            bin: Some("ollama"),
            url: None,
            install_hint: Some("brew install ollama  (or https://ollama.com)"),
            category: Local,
        },
        Agent {
            id: "lmstudio",
            name: "LM Studio",
            surface: Cli,
            bin: Some("lms"),
            url: Some("https://lmstudio.ai"),
            install_hint: Some(
                "Install LM Studio from https://lmstudio.ai (enables the `lms` CLI)",
            ),
            category: Local,
        },
        Agent {
            id: "openwebui",
            name: "Open WebUI",
            surface: Web,
            bin: None,
            url: Some("http://localhost:8080"),
            install_hint: Some("pip install open-webui && open-webui serve"),
            category: Local,
        },
        Agent {
            id: "localai",
            name: "LocalAI",
            surface: Web,
            bin: None,
            url: Some("http://localhost:8080"),
            install_hint: Some("https://localai.io"),
            category: Local,
        },
    ]
}

/// Look up an agent by id.
pub fn find(id: &str) -> Option<Agent> {
    let id = id.to_lowercase();
    registry().into_iter().find(|a| a.id == id)
}

/// Detection result for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub id: String,
    pub name: String,
    pub surface: String,
    pub category: String,
    pub installed: bool,
    pub version: Option<String>,
    pub url: Option<String>,
    pub install_hint: Option<String>,
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{bin}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
            let cmd = dir.join(format!("{bin}.cmd"));
            if cmd.is_file() {
                return Some(cmd);
            }
        }
    }
    None
}

/// Best-effort version string from `<bin> --version`.
fn detect_version(bin: &str) -> Option<String> {
    let out = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        None
    } else {
        Some(line.chars().take(60).collect())
    }
}

fn surface_str(s: Surface) -> &'static str {
    match s {
        Surface::Cli => "cli",
        Surface::Web => "web",
        Surface::Both => "both",
    }
}

fn category_str(c: Category) -> &'static str {
    match c {
        Category::Coding => "coding",
        Category::Research => "research",
        Category::Marketing => "marketing",
        Category::Local => "local",
        Category::Github => "github",
        Category::Generic => "generic",
    }
}

/// Detect install status for one agent.
pub fn status(agent: &Agent) -> AgentStatus {
    let installed_path = agent.bin.and_then(which);
    let installed = installed_path.is_some();
    let version = if installed {
        agent.bin.and_then(detect_version)
    } else {
        None
    };
    AgentStatus {
        id: agent.id.to_string(),
        name: agent.name.to_string(),
        surface: surface_str(agent.surface).to_string(),
        category: category_str(agent.category).to_string(),
        installed,
        version,
        url: agent.url.map(|u| u.to_string()),
        install_hint: agent.install_hint.map(|h| h.to_string()),
    }
}

/// Detect every agent.
pub fn detect_all() -> Vec<AgentStatus> {
    registry().iter().map(status).collect()
}

/// Pick the best agent id for a prompt (auto-router). Prefers installed tools.
pub fn route(prompt: &str) -> &'static str {
    let p = prompt.to_lowercase();
    let coding = [
        "fix",
        "bug",
        "refactor",
        "implement",
        "code",
        "function",
        "test",
        "build",
        "compile",
        "deploy",
        "error",
        ".rs",
        ".ts",
        ".py",
        "src/",
    ];
    let github = [
        "pull request",
        " pr ",
        "github",
        "commit",
        "branch",
        "merge",
        "issue",
    ];
    let marketing = [
        "landing",
        "marketing",
        "copy",
        "headline",
        "tagline",
        "tweet",
        "blog",
    ];
    let research = [
        "research",
        "explain",
        "compare",
        "summarize",
        "what is",
        "why",
        "how does",
    ];

    let has = |kws: &[&str]| kws.iter().any(|k| p.contains(k));

    let prefer = |ids: &[&'static str]| -> &'static str {
        for id in ids {
            if let Some(a) = find(id) {
                if a.bin.and_then(which).is_some() {
                    return id;
                }
            }
        }
        ids[0]
    };

    if has(&github) {
        prefer(&["gh", "copilot", "codex", "chatgpt"])
    } else if has(&coding) {
        prefer(&["claude", "cursor", "codex", "chatgpt"])
    } else if has(&marketing) {
        prefer(&["chatgpt", "gemini"])
    } else if has(&research) {
        prefer(&["chatgpt", "perplexity", "gemini"])
    } else {
        prefer(&["claude", "chatgpt"])
    }
}

/// Copy text to the OS clipboard. Returns the tool used, or an error string.
pub fn copy_to_clipboard(text: &str) -> Result<String, String> {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };

    for &(bin, args) in candidates {
        if which(bin).is_none() {
            continue;
        }
        use std::io::Write;
        let mut child = match std::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(stdin) = child.stdin.as_mut() {
            if stdin.write_all(text.as_bytes()).is_err() {
                continue;
            }
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        if ok {
            return Ok(bin.to_string());
        }
    }
    Err("no clipboard tool found (install pbcopy/clip/xclip/xsel/wl-copy)".to_string())
}

/// The shell command that runs a CLI agent with a prompt read from a temp file
/// (so multi-line prompts need no escaping). `None` for tools that can't take a
/// one-shot prompt (editors / local runtimes).
pub fn terminal_command(id: &str, bin: &str, prompt_file: &str) -> Option<String> {
    let read = format!("\"$(cat '{prompt_file}')\"");
    let cmd = match id {
        "claude" | "codex" | "windsurf" => format!("{bin} {read}"),
        "gemini" | "copilot" => format!("{bin} -p {read}"),
        "aider" => format!("{bin} --message {read}"),
        "opencode" => format!("{bin} run {read}"),
        // Editors / runtimes: no one-shot prompt arg.
        "cursor" | "ollama" | "lmstudio" | "gh" => return None,
        _ => format!("{bin} {read}"),
    };
    Some(cmd)
}

/// Structured result of a daemon-side launch (no terminal of its own).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchOutcome {
    /// "web" | "terminal" | "clipboard" | "error"
    pub method: String,
    pub agent: String,
    pub launched: bool,
    pub copied: bool,
    pub url: Option<String>,
    pub command: Option<String>,
    pub message: String,
}

fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Launch a prompt into an agent from a context with no terminal (the daemon).
///
/// - Web tools: copy the prompt to the clipboard and open the site.
/// - CLI tools (e.g. Claude Code): open a NEW terminal window running the agent
///   with the prompt — so it "renders into" the agent automatically.
/// - Editors / runtimes: copy to clipboard and open the app.
pub fn launch_detached(id: &str, prompt: &str, cwd: Option<&str>) -> LaunchOutcome {
    let agent = match find(id) {
        Some(a) => a,
        None => {
            return LaunchOutcome {
                method: "error".into(),
                agent: id.into(),
                launched: false,
                copied: false,
                url: None,
                command: None,
                message: format!("Unknown tool '{id}'."),
            }
        }
    };
    let st = status(&agent);
    let use_cli = matches!(agent.surface, Surface::Cli | Surface::Both) && st.installed;

    // Web path (web tools, or a CLI tool that isn't installed but has a URL).
    if !use_cli {
        if let Some(url) = agent.url {
            let copied = copy_to_clipboard(prompt).is_ok();
            let opened = open_url(url);
            let mut msg = format!(
                "Opened {} and copied the prompt to your clipboard — paste it in.",
                agent.name
            );
            if matches!(agent.surface, Surface::Cli) {
                msg = format!(
                    "{} CLI not installed; opened the web app instead. Prompt copied to clipboard.",
                    agent.name
                );
            }
            return LaunchOutcome {
                method: "web".into(),
                agent: agent.name.into(),
                launched: opened,
                copied,
                url: Some(url.to_string()),
                command: None,
                message: msg,
            };
        }
        return LaunchOutcome {
            method: "error".into(),
            agent: agent.name.into(),
            launched: false,
            copied: false,
            url: None,
            command: None,
            message: format!(
                "{} is not installed. Install it with: {}",
                agent.name,
                agent.install_hint.unwrap_or("(see the tool's site)")
            ),
        };
    }

    // CLI path: write prompt to a temp file, open a new terminal running the agent.
    let bin = agent.bin.expect("cli agent has a bin");
    let copied = copy_to_clipboard(prompt).is_ok();
    let prompt_file = match write_prompt_file(prompt) {
        Ok(p) => p,
        Err(e) => {
            return LaunchOutcome {
                method: "error".into(),
                agent: agent.name.into(),
                launched: false,
                copied,
                url: None,
                command: None,
                message: format!("Could not stage prompt: {e}"),
            }
        }
    };

    let Some(cmd) = terminal_command(agent.id, bin, &prompt_file) else {
        // Editors / runtimes: open the app, prompt is on the clipboard.
        let _ = open_app(bin, cwd);
        return LaunchOutcome {
            method: "clipboard".into(),
            agent: agent.name.into(),
            launched: true,
            copied,
            url: None,
            command: None,
            message: format!(
                "Opened {} — paste the prompt from your clipboard.",
                agent.name
            ),
        };
    };

    let full = match cwd {
        Some(d) => format!("cd {} && {cmd}", shell_quote(d)),
        None => cmd.clone(),
    };
    let launched = open_terminal(&full);
    LaunchOutcome {
        method: "terminal".into(),
        agent: agent.name.into(),
        launched,
        copied,
        url: None,
        command: Some(full.clone()),
        message: if launched {
            format!(
                "Launched {} in a new terminal with your compressed prompt.",
                agent.name
            )
        } else {
            format!("Could not open a terminal automatically. Run:\n  {full}")
        },
    }
}

fn write_prompt_file(prompt: &str) -> std::io::Result<String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("traceguard-prompt-{ts}.txt"));
    std::fs::write(&path, prompt)?;
    Ok(path.display().to_string())
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn open_url(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).status();
    r.map(|s| s.success()).unwrap_or(false)
}

fn open_app(bin: &str, cwd: Option<&str>) -> bool {
    let mut c = std::process::Command::new(bin);
    if let Some(d) = cwd {
        c.arg(d);
    }
    c.spawn().is_ok()
}

/// Open a new terminal window running `shell_cmd`.
fn open_terminal(shell_cmd: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
            applescript_escape(shell_cmd)
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", shell_cmd])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for term in ["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"] {
            if which(term).is_some()
                && std::process::Command::new(term)
                    .args(["-e", "bash", "-lc", shell_cmd])
                    .spawn()
                    .is_ok()
            {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_unique() {
        let ids: Vec<_> = registry().into_iter().map(|a| a.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }

    #[test]
    fn terminal_command_claude_reads_file() {
        let c = terminal_command("claude", "claude", "/tmp/p.txt").unwrap();
        assert!(c.contains("claude"));
        assert!(c.contains("cat '/tmp/p.txt'"));
    }

    #[test]
    fn terminal_command_none_for_editors() {
        assert!(terminal_command("cursor", "cursor", "/tmp/p.txt").is_none());
        assert!(terminal_command("ollama", "ollama", "/tmp/p.txt").is_none());
    }

    #[test]
    fn applescript_escaping() {
        assert_eq!(applescript_escape("a \"b\" \\c"), "a \\\"b\\\" \\\\c");
    }

    #[test]
    fn route_coding_prefers_known() {
        let r = route("fix the bug in src/main.rs");
        assert!(["claude", "cursor", "codex", "chatgpt"].contains(&r));
    }
}
