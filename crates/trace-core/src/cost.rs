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

    #[test]
    fn dated_model_id_resolves_via_substring() {
        // A dated/suffixed model id must still match its price row.
        let cost = estimate_cost(
            "anthropic",
            "claude-opus-4-20240620",
            Some(1_000_000),
            None,
            None,
        )
        .unwrap();
        // 1M input on opus = 15.0 (no output tokens).
        assert!((cost - 15.0).abs() < 1e-6, "got {cost}");
    }

    #[test]
    fn gpt_4o_mini_matches_before_gpt_4o() {
        // Table order lists gpt-4o-mini before gpt-4o; the more specific id must
        // win so mini isn't priced at the (much higher) gpt-4o rate.
        let mini = price_for("gpt-4o-mini").unwrap();
        assert_eq!(mini.input_per_mtok, 0.15);
        let base = price_for("gpt-4o").unwrap();
        assert_eq!(base.input_per_mtok, 2.5);
    }

    #[test]
    fn zero_input_tokens_is_zero_cost() {
        // Present-but-zero usage is honest data → a real 0.0, not "unavailable".
        assert_eq!(
            estimate_cost("anthropic", "claude-sonnet-4", Some(0), Some(0), Some(0)),
            Some(0.0)
        );
    }

    #[test]
    fn output_tokens_default_to_zero_when_absent() {
        // Input present, output absent: cost is input-only (not None).
        let cost = estimate_cost("anthropic", "claude-haiku", Some(2_000_000), None, None).unwrap();
        // 2M input on haiku = 2 * 0.8 = 1.6
        assert!((cost - 1.6).abs() < 1e-6, "got {cost}");
    }

    #[test]
    fn output_only_without_input_is_unavailable() {
        // Without input tokens we can't be honest about the number → None,
        // even though output tokens are known.
        assert_eq!(
            estimate_cost("openai", "gpt-4o", None, Some(1000), None),
            None
        );
    }

    #[test]
    fn cached_tokens_billed_at_cached_rate_as_subset_of_input() {
        // 1M input of which 400k is cached, plus 1M output, on sonnet:
        //   uncached input 600k * 3   = 1.8
        //   cached        400k * 0.3  = 0.12
        //   output          1M * 15   = 15.0
        // total = 16.92
        let cost = estimate_cost(
            "anthropic",
            "claude-sonnet-4",
            Some(1_000_000),
            Some(1_000_000),
            Some(400_000),
        )
        .unwrap();
        assert!((cost - 16.92).abs() < 1e-6, "got {cost}");
    }

    #[test]
    fn cached_exceeding_input_never_makes_uncached_negative() {
        // Defensive: even if reported cached > input, uncached clamps to 0 so
        // the input term can't go negative and net a smaller (or negative) bill.
        let cost = estimate_cost(
            "anthropic",
            "claude-sonnet-4",
            Some(100_000),
            None,
            Some(500_000),
        )
        .unwrap();
        // uncached = max(100k-500k,0)=0 → only cached 500k * 0.3/1M = 0.15
        assert!((cost - 0.15).abs() < 1e-6, "got {cost}");
    }

    #[test]
    fn negative_cached_is_clamped_to_zero() {
        // A bogus negative cached count must not credit the bill.
        let cost = estimate_cost(
            "anthropic",
            "claude-sonnet-4",
            Some(1_000_000),
            None,
            Some(-50),
        )
        .unwrap();
        // cached clamps to 0 → full 1M input * 3 = 3.0
        assert!((cost - 3.0).abs() < 1e-6, "got {cost}");
    }

    #[test]
    fn fractional_tokens_round_trip_without_loss() {
        // Small partial usage should produce a small, precise fractional cost.
        // 1234 input + 567 output on gpt-4o-mini:
        //   1234 * 0.15/1M + 567 * 0.6/1M = 0.0001851 + 0.0003402 = 0.0005253
        let cost = estimate_cost("openai", "gpt-4o-mini", Some(1234), Some(567), None).unwrap();
        assert!((cost - 0.0005253).abs() < 1e-9, "got {cost}");
    }

    #[test]
    fn local_provider_is_free_regardless_of_model_or_tokens() {
        for provider in ["local", "ollama", "llamacpp", "lmstudio"] {
            assert_eq!(
                estimate_cost(provider, "anything", Some(9_999_999), Some(9_999_999), None),
                Some(0.0),
                "provider {provider} should be free"
            );
        }
        // Casing doesn't matter.
        assert_eq!(
            estimate_cost("OLLAMA", "whatever", Some(1000), None, None),
            Some(0.0)
        );
    }

    #[test]
    fn multi_record_aggregation_sums_per_record_estimates() {
        // Mirrors how the daemon totals cost across many api_usage rows: sum the
        // per-record estimates, treating unknown-model records as unavailable
        // (skipped) rather than zero.
        let records: &[(&str, &str, Option<i64>, Option<i64>)] = &[
            (
                "anthropic",
                "claude-sonnet-4",
                Some(1_000_000),
                Some(1_000_000),
            ), // 18.0
            ("openai", "gpt-4o", Some(1_000_000), Some(0)), // 2.5
            ("openai", "mystery-model", Some(1_000_000), Some(1_000_000)), // None
        ];
        let mut total = 0.0;
        let mut any_unavailable = false;
        for (prov, model, inp, out) in records {
            match estimate_cost(prov, model, *inp, *out, None) {
                Some(c) => total += c,
                None => any_unavailable = true,
            }
        }
        assert!((total - 20.5).abs() < 1e-6, "got {total}");
        assert!(any_unavailable);
    }
}
