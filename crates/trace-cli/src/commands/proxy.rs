//! `trc proxy` — opt-in instructions for the loopback provider proxy.
//!
//! The daemon owns the actual reverse proxy. This command only starts/attaches
//! to that local daemon and prints the provider-specific base URL plus the
//! metadata-only attribution header. It never reads or accepts an API key.

use anyhow::{bail, Result};
use serde_json::json;
use trace_daemon::proxy::Provider;

use crate::daemon_ctl;

/// Print the local proxy endpoint and its honest capture boundaries.
pub fn run(provider_name: &str, run_id: Option<&str>, json_output: bool) -> Result<()> {
    let provider = Provider::parse(provider_name).ok_or_else(|| {
        anyhow::anyhow!(
            "unsupported proxy provider `{provider_name}`; choose openai, anthropic, or google"
        )
    })?;
    let slug = match provider {
        Provider::OpenAi => "openai",
        Provider::Anthropic => "anthropic",
        Provider::Google => "google",
    };
    let pricing_source = match provider {
        Provider::OpenAi => "https://developers.openai.com/api/docs/pricing",
        Provider::Anthropic => "https://platform.claude.com/docs/en/about-claude/pricing",
        Provider::Google => "https://ai.google.dev/gemini-api/docs/pricing",
    };
    if let Some(run_id) = run_id {
        if run_id.trim().is_empty() {
            bail!("--run-id cannot be empty");
        }
    }
    let port = daemon_ctl::ensure_running()?;
    let base_url = format!("http://127.0.0.1:{port}/proxy/{slug}");
    let payload = json!({
        "provider": slug,
        "proxy_base_url": base_url,
        "proxy_origin": format!("http://127.0.0.1:{port}"),
        "run_header": "X-Trace-Run-ID",
        "run_id": run_id,
        "request_content": "forwarded in memory only",
        "response_mode": "buffered and passed through",
        "cost": "Trace API-equivalent estimate from provider-reported tokens",
        "pricing_source": pricing_source,
        "pricing_version": trace_core::cost::pricing_version(),
        "invoice": false,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("Trace proxy ready ({slug})");
    println!("  base URL:  {base_url}");
    println!("  upstream:  provider HTTPS endpoint (fixed by provider)");
    println!("  attribution: add X-Trace-Run-ID: <run-id> when the client supports custom headers");
    if let Some(run_id) = run_id {
        println!("  run:       {run_id}");
    }
    println!("  capture:   provider-reported tokens only; no prompt/response text is stored");
    println!(
        "  estimate:  API-equivalent price-table estimate, not an invoice or subscription bill"
    );
    println!(
        "  pricing:   {pricing_source} (active catalog {})",
        trace_core::cost::pricing_version()
    );
    println!("  note:      responses are buffered before pass-through so usage can be read");
    Ok(())
}
