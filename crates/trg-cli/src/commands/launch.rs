//! `trg agents`, `trg use <target>`, `trg launch <target> "<prompt>"`.
//!
//! Launches a prompt into a real AI tool: CLI tools are detected and executed;
//! web tools get the prompt on the clipboard plus the site opened. No fake
//! success — if a tool is missing, you get the exact install command.

use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};
use traceguard_core::{agents, paths, secrets};

use crate::settings;

/// `trg agents` — list every supported tool and whether it is installed.
pub fn agents() -> Result<()> {
    let all = agents::detect_all();
    let default = settings::load().default_target;
    println!("Supported AI tools ({} total):\n", all.len());
    println!("  {:<14} {:<7} {:<10} DETAIL", "ID", "SURFACE", "STATUS");
    for a in &all {
        let status = if a.surface == "web" {
            "web".to_string()
        } else if a.installed {
            format!(
                "installed{}",
                if default.as_deref() == Some(a.id.as_str()) {
                    " *"
                } else {
                    ""
                }
            )
        } else {
            "missing".to_string()
        };
        let detail = a
            .version
            .clone()
            .or_else(|| a.url.clone())
            .unwrap_or_default();
        println!("  {:<14} {:<7} {:<10} {}", a.id, a.surface, status, detail);
    }
    if let Some(d) = default {
        println!("\nDefault launch target: {d}");
    } else {
        println!("\nNo default set. Use `trg use <id>` to set one.");
    }
    println!("`*` = current default.  Launch with `trg launch <id> \"<prompt>\"`.");
    Ok(())
}

/// `trg use <id>` — set the default launch target.
pub fn use_target(id: &str) -> Result<()> {
    let agent = agents::find(id)
        .ok_or_else(|| anyhow!("unknown tool '{id}'. Run `trg agents` to see valid ids."))?;
    let mut s = settings::load();
    s.default_target = Some(agent.id.to_string());
    settings::save(&s)?;
    println!(
        "Default launch target set to {} ({}).",
        agent.id, agent.name
    );
    Ok(())
}

pub struct LaunchOptions {
    pub target: Option<String>,
    pub prompt: String,
    pub yes: bool,
}

/// `trg launch [target] "<prompt>"` — send a prompt to a tool.
pub fn launch(opts: LaunchOptions) -> Result<()> {
    let prompt = opts.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(anyhow!(
            "no prompt provided. Usage: trg launch <target> \"<prompt>\""
        ));
    }

    // Secret scan before anything leaves the prompt box.
    let findings = secrets::scan_text(&prompt);
    if !findings.is_empty() && !opts.yes {
        println!("⚠ This prompt may contain secrets:");
        for f in &findings {
            println!("   {} ({})", f.secret_type, f.redacted_value);
        }
        println!("Re-run with --yes to continue anyway, or remove the secret first.");
        return Ok(());
    }

    // Resolve the target: explicit → default → auto-route.
    let target = match opts.target.as_deref() {
        Some("auto") => agents::route(&prompt).to_string(),
        Some(t) => t.to_string(),
        None => match settings::load().default_target {
            Some(d) => d,
            None => {
                let routed = agents::route(&prompt);
                println!("No target given and no default set; auto-routing to '{routed}'.");
                routed.to_string()
            }
        },
    };

    let agent = agents::find(&target)
        .ok_or_else(|| anyhow!("unknown tool '{target}'. Run `trg agents` to see valid ids."))?;
    let st = agents::status(&agent);

    // Decide CLI vs web.
    let use_cli =
        matches!(agent.surface, agents::Surface::Cli | agents::Surface::Both) && st.installed;

    if use_cli {
        launch_cli(&agent, &prompt)
    } else if agent.url.is_some() {
        if matches!(agent.surface, agents::Surface::Cli) && !st.installed {
            println!(
                "{} CLI not found. Falling back to the web app.\n  Install the CLI with: {}",
                agent.name,
                agent.install_hint.unwrap_or("(see the tool's site)")
            );
        }
        launch_web(&agent, &prompt)
    } else {
        Err(anyhow!(
            "TraceGuard could not find {}.\nInstall it with:\n  {}\nor use a web tool, e.g.: trg launch chatgpt \"<prompt>\"",
            agent.name,
            agent.install_hint.unwrap_or("(see the tool's site)")
        ))
    }
}

/// Build the argv for a known CLI tool. Returns None for clipboard-style tools
/// (editors / local runtimes that can't take a one-shot prompt arg).
fn cli_argv(id: &str, bin: &str, prompt: &str) -> Option<Vec<String>> {
    let v = |parts: &[&str]| Some(parts.iter().map(|s| s.to_string()).collect());
    match id {
        "claude" => v(&[bin, prompt]),
        "codex" => v(&[bin, prompt]),
        "gemini" => v(&[bin, "-p", prompt]),
        "copilot" => v(&[bin, "-p", prompt]),
        "aider" => v(&[bin, "--message", prompt]),
        "opencode" => v(&[bin, "run", prompt]),
        // Editors / runtimes: open the tool, prompt goes via clipboard.
        "cursor" | "windsurf" | "ollama" | "lmstudio" | "gh" => None,
        _ => v(&[bin, prompt]),
    }
}

fn launch_cli(agent: &agents::Agent, prompt: &str) -> Result<()> {
    let bin = agent.bin.expect("cli agent has a bin");
    match cli_argv(agent.id, bin, prompt) {
        Some(argv) => {
            println!("Launching {} …\n  $ {}", agent.name, shell_join(&argv));
            let status = Command::new(&argv[0])
                .args(&argv[1..])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| {
                    anyhow!(
                        "could not start {}: {e}\nInstall or fix it with:\n  {}",
                        agent.name,
                        agent.install_hint.unwrap_or("(see the tool's site)")
                    )
                })?;
            if !status.success() {
                println!(
                    "\n{} exited with a non-zero status. If it does not accept a one-shot prompt, the prompt is on your clipboard — paste it in.",
                    agent.name
                );
                let _ = agents::copy_to_clipboard(prompt);
            }
            Ok(())
        }
        None => {
            // Clipboard + open the editor/runtime.
            let copied = agents::copy_to_clipboard(prompt).is_ok();
            println!("{} does not accept a one-shot prompt argument.", agent.name);
            if copied {
                println!(
                    "Your prompt is on the clipboard — paste it into {}.",
                    agent.name
                );
            }
            match agent.id {
                "cursor" | "windsurf" => {
                    let _ = Command::new(agent.bin.unwrap()).arg(".").status();
                    println!("Opened {} in the current directory.", agent.name);
                }
                "ollama" => println!("Run a local model: ollama run <model>  (then paste the prompt)"),
                "lmstudio" => println!("Open LM Studio and paste the prompt into a chat."),
                "gh" => println!("`gh` is not a chat tool — use `trg launch copilot \"<prompt>\"` for GitHub Copilot."),
                _ => {}
            }
            Ok(())
        }
    }
}

fn launch_web(agent: &agents::Agent, prompt: &str) -> Result<()> {
    let url = agent.url.expect("web agent has a url");
    let copied = agents::copy_to_clipboard(prompt);
    match &copied {
        Ok(tool) => println!("Prompt copied to clipboard (via {tool})."),
        Err(e) => println!("Could not copy to clipboard: {e}\nCopy the prompt manually."),
    }
    match open::that(url) {
        Ok(_) => println!("Opened {} ({url}).", agent.name),
        Err(e) => println!("Could not open a browser ({e}). Visit: {url}"),
    }
    println!(
        "Paste the prompt ({}) into {}.",
        if copied.is_ok() {
            "on your clipboard"
        } else {
            "shown above"
        },
        agent.name
    );
    Ok(())
}

fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Used by `trg doctor` to report the config/settings path.
pub fn settings_path_display() -> String {
    paths::global_dir()
        .map(|d| d.join("settings.json").display().to_string())
        .unwrap_or_else(|_| "~/.traceguard/settings.json".into())
}
