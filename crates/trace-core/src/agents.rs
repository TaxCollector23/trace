//! AI tool registry and detection.
//!
//! Used by `trace doctor` to report which AI coding tools are installed on the
//! machine, and by the clipboard helper shared with other commands.

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
}
