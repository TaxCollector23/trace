//! Local cost estimation for AI/API usage.
//!
//! Cost tracking supports partial data. When token counts or a price for the
//! model are unavailable, the estimate is `None` and the UI must label it
//! "unavailable" rather than guessing.

use serde::{Deserialize, Serialize};

/// Per-million-token USD prices for a model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    /// Price for cached input tokens, when the provider distinguishes them.
    pub cached_input_per_mtok: f64,
}

/// Look up published pricing for a model id. Matching is prefix/substring based
/// so that dated model ids (e.g. `claude-...-20240620`) still resolve.
pub fn price_for(model: &str) -> Option<ModelPrice> {
    let m = model.to_lowercase();
    // (substring, price). First match wins; order specific before generic.
    let table: &[(&str, ModelPrice)] = &[
        (
            "claude-opus",
            ModelPrice {
                input_per_mtok: 15.0,
                output_per_mtok: 75.0,
                cached_input_per_mtok: 1.5,
            },
        ),
        (
            "claude-sonnet",
            ModelPrice {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cached_input_per_mtok: 0.3,
            },
        ),
        (
            "claude-haiku",
            ModelPrice {
                input_per_mtok: 0.8,
                output_per_mtok: 4.0,
                cached_input_per_mtok: 0.08,
            },
        ),
        (
            "gpt-4o-mini",
            ModelPrice {
                input_per_mtok: 0.15,
                output_per_mtok: 0.6,
                cached_input_per_mtok: 0.075,
            },
        ),
        (
            "gpt-4o",
            ModelPrice {
                input_per_mtok: 2.5,
                output_per_mtok: 10.0,
                cached_input_per_mtok: 1.25,
            },
        ),
        (
            "gpt-4.1",
            ModelPrice {
                input_per_mtok: 2.0,
                output_per_mtok: 8.0,
                cached_input_per_mtok: 0.5,
            },
        ),
        (
            "o1",
            ModelPrice {
                input_per_mtok: 15.0,
                output_per_mtok: 60.0,
                cached_input_per_mtok: 7.5,
            },
        ),
        (
            "gemini-1.5-pro",
            ModelPrice {
                input_per_mtok: 1.25,
                output_per_mtok: 5.0,
                cached_input_per_mtok: 0.3125,
            },
        ),
        (
            "gemini-1.5-flash",
            ModelPrice {
                input_per_mtok: 0.075,
                output_per_mtok: 0.3,
                cached_input_per_mtok: 0.01875,
            },
        ),
        (
            "gemini",
            ModelPrice {
                input_per_mtok: 0.5,
                output_per_mtok: 1.5,
                cached_input_per_mtok: 0.125,
            },
        ),
    ];
    table
        .iter()
        .find(|(needle, _)| m.contains(needle))
        .map(|(_, price)| *price)
}

/// Local models are free / unmetered.
pub fn is_local_provider(provider: &str) -> bool {
    let p = provider.to_lowercase();
    matches!(p.as_str(), "local" | "ollama" | "llamacpp" | "lmstudio")
}

/// Estimate cost in USD. Returns `None` when there is not enough data to be
/// honest about the number. Cached tokens are billed at the cached rate and are
/// assumed to be a subset of the input tokens.
pub fn estimate_cost(
    provider: &str,
    model: &str,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cached_tokens: Option<i64>,
) -> Option<f64> {
    if is_local_provider(provider) {
        return Some(0.0);
    }
    let price = price_for(model)?;
    let input = input_tokens? as f64;
    let output = output_tokens.unwrap_or(0) as f64;
    let cached = cached_tokens.unwrap_or(0).max(0) as f64;
    let uncached_input = (input - cached).max(0.0);

    let cost = uncached_input / 1_000_000.0 * price.input_per_mtok
        + cached / 1_000_000.0 * price.cached_input_per_mtok
        + output / 1_000_000.0 * price.output_per_mtok;
    Some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_is_free() {
        assert_eq!(
            estimate_cost("ollama", "llama3", Some(1000), Some(1000), None),
            Some(0.0)
        );
    }

    #[test]
    fn unknown_model_is_unavailable() {
        assert_eq!(
            estimate_cost("openai", "mystery-model", Some(1000), Some(1000), None),
            None
        );
    }

    #[test]
    fn missing_tokens_is_unavailable() {
        assert_eq!(
            estimate_cost("anthropic", "claude-sonnet-4", None, None, None),
            None
        );
    }

    #[test]
    fn computes_anthropic_cost() {
        // 1M input + 1M output on sonnet = 3 + 15 = 18.0
        let cost = estimate_cost(
            "anthropic",
            "claude-sonnet-4",
            Some(1_000_000),
            Some(1_000_000),
            None,
        )
        .unwrap();
        assert!((cost - 18.0).abs() < 1e-6);
    }
}
