//! `trc pricing` — inspect and refresh the versioned cost catalog.

use anyhow::Result;
use trace_core::cost;

use crate::colors;

pub fn show(json: bool) -> Result<()> {
    let active = cost::load_active_catalog_info()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&active.catalog)?);
    } else {
        println!("Pricing version: {}", active.catalog.pricing_version);
        println!("Effective from:  {}", active.catalog.effective_from);
        println!("Source:          {}", active.origin.as_str());
        println!("Models:          {}", active.catalog.models.len());
        println!();
        println!("Sources:");
        for source in &active.catalog.sources {
            println!(
                "  {} — {} (retrieved {})",
                source.provider, source.url, source.retrieved_at
            );
        }
        println!();
        println!("Models:");
        for model in &active.catalog.models {
            println!("  {}/{}", model.provider, model.model);
            for tier in &model.tiers {
                let minimum = tier.min_input_tokens.map_or_else(
                    || "default".to_string(),
                    |tokens| format!(">={tokens} input tokens"),
                );
                println!(
                    "    {}: {minimum}; input ${:.6}/M, output ${:.6}/M, cache read {}, cache write {}",
                    tier.name,
                    tier.input_per_mtok,
                    tier.output_per_mtok,
                    display_rate(tier.cache_read_per_mtok),
                    display_write_rates(&tier.cache_write_per_mtok),
                );
            }
        }
    }
    if let Some(warning) = active.warning {
        eprintln!("{} {}", colors::yellow("warning:"), warning);
    }
    Ok(())
}

pub fn update() -> Result<()> {
    let outcome = cost::update_pricing_from_litellm()?;
    if outcome.used_remote {
        println!(
            "Updated pricing catalog to {} at {}",
            outcome.catalog.pricing_version,
            outcome.path.display()
        );
        println!(
            "LiteLLM is a structural update source; verify provider rates before treating new rows as official."
        );
    } else {
        println!(
            "Pricing update unavailable; keeping pinned catalog {}",
            outcome.catalog.pricing_version
        );
    }
    if let Some(warning) = outcome.warning {
        eprintln!("{} {}", colors::yellow("warning:"), warning);
    }
    Ok(())
}

fn display_rate(rate: Option<f64>) -> String {
    rate.map_or_else(
        || "not published".to_string(),
        |rate| format!("${rate:.6}/M"),
    )
}

fn display_write_rates(rates: &std::collections::BTreeMap<String, f64>) -> String {
    if rates.is_empty() {
        return "not published".to_string();
    }
    rates
        .iter()
        .map(|(ttl, rate)| format!("{ttl} ${rate:.6}/M"))
        .collect::<Vec<_>>()
        .join(", ")
}
