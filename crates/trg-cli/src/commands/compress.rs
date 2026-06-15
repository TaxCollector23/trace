//! `trg compress-prompt` — locally and deterministically compress a prompt
//! before you send it to an agent, reducing token usage while preserving
//! constraints, commands, file names, and acceptance criteria.
//!
//! Compression is local by default; no prompt text leaves the machine. Nothing
//! is sent to any agent automatically — you review and accept the result.

use std::io::{Read, Write};

use anyhow::Result;
use traceguard_core::models::NewPromptCompression;
use traceguard_core::prompt::{self, OutputBudget};

use crate::client::Client;
use crate::daemon_ctl;
use crate::project;

pub struct CompressOptions {
    /// Prompt text from the CLI; if empty, read from stdin (interactive paste).
    pub prompt: String,
    /// Append an output-budget guidance block to the compressed prompt.
    pub budget: bool,
    /// Soft output token target to include in the budget block.
    pub target: Option<usize>,
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

    let result = prompt::compress(&prompt_text);

    let mut final_prompt = result.compressed.clone();
    if opts.budget {
        let budget = OutputBudget {
            concise: true,
            no_repeat_unchanged_code: true,
            summarize_in_bullets: true,
            only_show_changed_files: true,
            no_full_files_unless_necessary: true,
            ask_before_large_rewrites: true,
            target_tokens: opts.target,
        };
        let block = budget.to_instruction_block();
        if !block.is_empty() {
            final_prompt = format!("{final_prompt}\n\n{block}");
        }
    }

    println!("\n── Compressed prompt ──");
    println!("{final_prompt}");
    println!("\n── Estimates (local, approximate) ──");
    println!("  original:   ~{} tokens", result.original_tokens);
    println!("  compressed: ~{} tokens", result.compressed_tokens);
    println!("  reduction:  ~{:.0}%", result.reduction_pct);

    // Accept / reject. Never auto-uses the prompt.
    let accept = if opts.yes {
        true
    } else {
        prompt_accept("Accept this compressed prompt?")?
    };
    if !accept {
        println!("Rejected. Original prompt left unchanged.");
        return Ok(());
    }

    // Record stats locally (best-effort). Text only when prompt history is on.
    record(&prompt_text, &final_prompt, &result);

    println!("\nAccepted. Copy the compressed prompt above into your agent.");
    Ok(())
}

fn record(original: &str, final_prompt: &str, result: &prompt::CompressionResult) {
    // Determine whether to store prompt text. Default true; respect project config.
    let store_text = project::load_current()
        .map(|p| p.config.prompt_history)
        .unwrap_or(true);

    let Ok(port) = daemon_ctl::ensure_running() else {
        return;
    };
    let client = Client::new(port);
    let _ = client.post(
        "/api/prompt-compressions",
        &NewPromptCompression {
            run_id: None,
            original_tokens: result.original_tokens as i64,
            compressed_tokens: result.compressed_tokens as i64,
            reduction_pct: result.reduction_pct,
            original_prompt: store_text.then(|| original.to_string()),
            final_prompt: store_text.then(|| final_prompt.to_string()),
        },
    );
}

fn read_stdin_prompt() -> Result<String> {
    println!("Paste your prompt, then press Ctrl-D (Ctrl-Z on Windows) to finish:");
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn prompt_accept(question: &str) -> Result<bool> {
    print!("{question} [y] accept  [n] reject: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}
