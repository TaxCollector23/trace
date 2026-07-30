//! `trg compress-prompt` — TraceCompress: compress a prompt locally before
//! sending it to an agent.
//!
//! Compression is local and deterministic. Nothing is sent to any model, and
//! nothing is sent to your agent automatically — you review and accept the
//! result. Token counts are estimates and labelled as such.

use std::io::{Read, Write};

use anyhow::Result;
use traceguard_core::ids::short_hash;
use traceguard_core::models::NewPromptCompression;
use traceguard_core::prompt::{self, CompressionMode, CompressionResult, OutputBudgetPreset};

use crate::client::Client;
use crate::daemon_ctl;
use crate::project;

pub struct CompressOptions {
    /// Prompt text from the CLI; if empty, read from stdin (interactive paste).
    pub prompt: String,
    /// "normal" | "concise" | "bare". None → project default.
    pub mode: Option<String>,
    /// Optional output-budget preset: tiny | short | normal | detailed.
    pub output_budget: Option<String>,
    /// Accept without prompting (non-interactive).
    pub yes: bool,
}

pub fn run(opts: CompressOptions) -> Result<()> {
    let prompt_text = if opts.prompt.trim().is_empty() {
        read_stdin_prompt()?
    } else {
        opts.prompt.clone()
    };
    if prompt_text.trim().is_empty() {
        println!("No prompt provided. Nothing to compress.");
        return Ok(());
    }

    let mode = resolve_mode(opts.mode.as_deref());
    let result = prompt::compress_with_mode(&prompt_text, mode);

    print_result(&result, opts.output_budget.as_deref());

    // Conflicts: warn and (if interactive) require acknowledgement.
    if !result.conflicts.is_empty()
        && !opts.yes
        && !confirm("Proceed despite the conflict(s) above?")?
    {
        println!("Aborted. Resolve the conflicting instructions and try again.");
        return Ok(());
    }

    let accept = opts.yes || confirm("Accept this compressed prompt?")?;
    if !accept {
        println!("Rejected. Original prompt left unchanged.");
        return Ok(());
    }

    record(&result, None);
    println!("\nAccepted. Copy the compressed prompt above into your agent.");
    Ok(())
}

/// Resolve the mode from a flag, or the project's configured default.
pub fn resolve_mode(flag: Option<&str>) -> CompressionMode {
    if let Some(m) = flag {
        return CompressionMode::parse(m);
    }
    let default = project::load_current()
        .map(|p| p.config.prompt_compression.default_mode)
        .unwrap_or_else(|_| "concise".to_string());
    CompressionMode::parse(&default)
}

/// Print the full TraceCompress result, optionally appending an output budget.
pub fn print_result(result: &CompressionResult, output_budget: Option<&str>) {
    println!("\n── Compressed prompt ({} mode) ──", result.mode);
    println!("{}", result.compressed);

    if let Some(preset) = output_budget {
        let block = OutputBudgetPreset::parse(preset).to_instruction_block();
        println!("\n{block}");
    }

    println!("\n── Response rules (attach to the agent prompt) ──");
    println!("{}", result.response_rules);

    println!("\n── Estimates (local, approximate) ──");
    println!("  original:   ~{} tokens", result.original_tokens);
    println!("  compressed: ~{} tokens", result.compressed_tokens);
    println!("  reduction:  ~{:.0}%", result.reduction_pct);

    if !result.preserved_constraints.is_empty() {
        println!("\n── Preserved constraints ──");
        for c in &result.preserved_constraints {
            println!("  ✓ {c}");
        }
    }
    if !result.removed_redundancy.is_empty() {
        println!("\n── Removed redundancy ──");
        for r in &result.removed_redundancy {
            println!("  - {r}");
        }
    }
    if !result.conflicts.is_empty() {
        println!("\n⚠ Possible conflicts (resolve before relying on the compression):");
        for c in &result.conflicts {
            println!("  ! {c}");
        }
    }
}

/// Build a storable record, honoring the project's prompt-history setting.
/// Token estimates and metadata are always stored; raw text only when enabled.
pub fn build_record(result: &CompressionResult, run_id: Option<String>) -> NewPromptCompression {
    let (project_id, store_text) = match project::load_current() {
        Ok(p) => {
            let pid = current_project_id(&p.root);
            (pid, p.config.prompt_compression.prompt_history)
        }
        Err(_) => (None, true),
    };

    NewPromptCompression {
        run_id,
        project_id,
        mode: result.mode.clone(),
        original_token_estimate: result.original_tokens as i64,
        compressed_token_estimate: result.compressed_tokens as i64,
        estimated_reduction_percent: result.reduction_pct,
        compressed_prompt_hash: short_hash(&result.compressed),
        original_prompt_stored: store_text,
        compressed_prompt_stored: store_text,
        preserved_constraints_json: Some(
            serde_json::to_string(&result.preserved_constraints).unwrap_or_default(),
        ),
        removed_redundancy_json: Some(
            serde_json::to_string(&result.removed_redundancy).unwrap_or_default(),
        ),
        original_prompt: store_text.then(|| result.original.clone()),
        compressed_prompt: store_text.then(|| result.compressed.clone()),
    }
}

/// Persist a compression record to the daemon (best-effort).
pub fn record(result: &CompressionResult, run_id: Option<String>) {
    let Ok(port) = daemon_ctl::ensure_running() else {
        return;
    };
    let client = Client::new(port);
    let _ = client.post(
        "/api/prompt-compressor/record",
        &build_record(result, run_id),
    );
}

fn current_project_id(root: &std::path::Path) -> Option<String> {
    let port = daemon_ctl::running_port()?;
    let client = Client::new(port);
    let projects: Vec<traceguard_core::models::Project> = client.get_json("/api/projects").ok()?;
    let target = root.display().to_string();
    projects
        .into_iter()
        .find(|p| p.path == target)
        .map(|p| p.id)
}

fn read_stdin_prompt() -> Result<String> {
    println!("Paste your prompt, then press Ctrl-D (Ctrl-Z on Windows) to finish:");
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn confirm(question: &str) -> Result<bool> {
    print!("{question} [y/N]: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}
