//! `trg guard "<prompt>"` — harden a prompt: clean it, attach strict rules,
//! score it, and lint it. Optionally tailor to the project and launch it.

use anyhow::{anyhow, Result};
use traceguard_core::harden::{self, HardenMode};
use traceguard_core::{agents, paths, scan};

use crate::commands::launch::{self, LaunchOptions};
use crate::project;

pub struct GuardOptions {
    pub prompt: String,
    pub mode: Option<String>,
    pub coding: bool,
    pub project: bool,
    pub copy: bool,
    pub launch: Option<String>,
    pub yes: bool,
}

pub fn run(opts: GuardOptions) -> Result<()> {
    if opts.prompt.trim().is_empty() {
        return Err(anyhow!("no prompt provided. Usage: trg guard \"<prompt>\""));
    }

    // Mode: --coding implies coding mode unless an explicit mode is given.
    let mode = match (&opts.mode, opts.coding) {
        (Some(m), _) => HardenMode::parse(m),
        (None, true) => HardenMode::Coding,
        (None, false) => HardenMode::parse(&default_mode()),
    };

    // Project context (from --project scan).
    let context = if opts.project {
        let root = project::load_current()
            .map(|p| p.root)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        Some(scan::scan(&root).to_context())
    } else {
        None
    };

    let custom_rules = load_custom_rules();
    let result = harden::harden(&opts.prompt, mode, context.as_deref(), &custom_rules);

    // Output.
    println!("── Hardened prompt ({} mode) ──\n", result.mode);
    println!("{}\n", result.hardened);
    println!("── Prompt score ──");
    print_score("before", &result.score_before);
    print_score("after ", &result.score_after);
    if !result.lint.is_empty() {
        println!("\n── Lint warnings ──");
        for w in &result.lint {
            println!("  ! {w}");
        }
    }

    if opts.copy {
        match agents::copy_to_clipboard(&result.hardened) {
            Ok(t) => println!("\nCopied hardened prompt to clipboard (via {t})."),
            Err(e) => println!("\nCould not copy to clipboard: {e}"),
        }
    }

    if let Some(target) = opts.launch {
        println!();
        launch::launch(LaunchOptions {
            target: Some(target),
            prompt: result.hardened.clone(),
            yes: opts.yes,
        })?;
    }

    Ok(())
}

fn print_score(label: &str, s: &harden::PromptScore) {
    let bars: Vec<String> = s
        .metrics
        .iter()
        .map(|m| format!("{} {}", m.name, m.value))
        .collect();
    println!("  {label}: {}/100   ({})", s.overall, bars.join(", "));
}

fn default_mode() -> String {
    crate::settings::load()
        .default_mode
        .unwrap_or_else(|| "balanced".to_string())
}

/// Custom rules from `~/.traceguard/rules.json` (a JSON array of strings).
fn load_custom_rules() -> Vec<String> {
    let Ok(dir) = paths::global_dir() else {
        return vec![];
    };
    let p = dir.join("rules.json");
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

/// `trg rules add/list/clear` — manage custom hardening rules.
pub fn rules(action: RulesAction) -> Result<()> {
    let dir = paths::ensure_global_dir()?;
    let p = dir.join("rules.json");
    let mut rules = load_custom_rules();
    match action {
        RulesAction::List => {
            if rules.is_empty() {
                println!("No custom rules. Add one with: trg rules add \"<rule>\"");
            }
            for (i, r) in rules.iter().enumerate() {
                println!("  {}. {r}", i + 1);
            }
        }
        RulesAction::Add(rule) => {
            rules.push(rule.clone());
            std::fs::write(&p, serde_json::to_string_pretty(&rules)?)?;
            println!("Added rule: {rule}");
        }
        RulesAction::Clear => {
            std::fs::write(&p, "[]")?;
            println!("Cleared all custom rules.");
        }
    }
    Ok(())
}

pub enum RulesAction {
    List,
    Add(String),
    Clear,
}
