//! Versioned provider pricing and honest usage-event cost computation.
//!
//! The embedded catalog is data, not a second pricing table in Rust. Every
//! row carries an effective timestamp and the catalog records the provider
//! page and retrieval date that support it. A calculation is `None` when a
//! required rate, token count, cache tier, model, or timestamp is unknown.
//! Reported provider costs are never overwritten: when both values exist, the
//! reported number remains the event cost and the computed number is retained
//! for reconciliation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths;

/// URL for the community-maintained structural update source used by
/// `trc pricing update`. It is not an official provider source; the pinned
/// catalog remains the fallback and all pinned rows cite official pages.
pub const LITELLM_PRICE_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

const EMBEDDED_PRICING_JSON: &str = include_str!("../data/pricing.json");
const RECONCILIATION_TOLERANCE_USD: f64 = 0.000001;

/// One official source citation recorded in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PricingSource {
    pub provider: String,
    pub url: String,
    pub retrieved_at: String,
    pub notes: String,
}

/// One price tier for a model. All numeric rates are USD per million tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceTier {
    pub name: String,
    #[serde(default)]
    pub min_input_tokens: Option<u64>,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    #[serde(default)]
    pub cache_read_per_mtok: Option<f64>,
    #[serde(default)]
    pub cache_write_per_mtok: BTreeMap<String, f64>,
    /// `output` means reasoning tokens are already included in output_tokens.
    /// `none` means the provider does not expose a separately billable
    /// reasoning component for this row.
    pub reasoning_billing: String,
    #[serde(default)]
    pub notes: Option<String>,
}

impl PriceTier {
    fn cache_write_rate(&self, ttl: Option<&str>) -> Option<f64> {
        if self.cache_write_per_mtok.is_empty() {
            return None;
        }
        match ttl {
            Some(ttl) => self
                .cache_write_per_mtok
                .get(ttl)
                .or_else(|| self.cache_write_per_mtok.get(&normalize_cache_ttl(ttl)))
                .copied(),
            None if self.cache_write_per_mtok.len() == 1 => {
                self.cache_write_per_mtok.values().next().copied()
            }
            None => None,
        }
    }
}

fn normalize_cache_ttl(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "5m" | "5min" | "5-minute" | "ephemeral-5m" => "5m".to_string(),
        "1h" | "1hr" | "1-hour" | "ephemeral-1h" => "1h".to_string(),
        other => other.to_string(),
    }
}

/// A versioned provider/model record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPricing {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub effective_from: String,
    pub tiers: Vec<PriceTier>,
}

/// The complete versioned pricing catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PricingCatalog {
    #[serde(default)]
    pub description: Option<String>,
    pub schema_version: u32,
    pub pricing_version: String,
    pub effective_from: String,
    pub sources: Vec<PricingSource>,
    pub models: Vec<ModelPricing>,
}

/// The model and catalog row selected for an event timestamp.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModel {
    pub provider: String,
    pub model: String,
    pub effective_from: String,
    pub pricing: ModelPricing,
}

impl PricingCatalog {
    /// Parse and validate a catalog before it can be used or persisted.
    pub fn from_json(json: &str) -> Result<Self> {
        let catalog: Self = serde_json::from_str(json).context("invalid pricing catalog JSON")?;
        catalog.validate()?;
        Ok(catalog)
    }

    /// Validate schema, timestamps, sources, and every numeric rate.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            anyhow::bail!("unsupported pricing catalog schema {}", self.schema_version);
        }
        if self.pricing_version.trim().is_empty() {
            anyhow::bail!("pricing_version is empty");
        }
        parse_timestamp(&self.effective_from)
            .with_context(|| format!("invalid catalog effective_from {}", self.effective_from))?;
        if self.sources.is_empty() {
            anyhow::bail!("pricing catalog has no sources");
        }
        for (index, source) in self.sources.iter().enumerate() {
            if source.provider.trim().is_empty() || source.url.trim().is_empty() {
                anyhow::bail!("pricing source {index} is missing provider or url");
            }
            if source.retrieved_at.trim().is_empty() {
                anyhow::bail!("pricing source {index} is missing retrieved_at");
            }
        }
        if self.models.is_empty() {
            anyhow::bail!("pricing catalog has no models");
        }
        for (index, model) in self.models.iter().enumerate() {
            let provider = normalize_provider(&model.provider)
                .ok_or_else(|| anyhow!("model {index} has unknown provider"))?;
            if model.model.trim().is_empty() {
                anyhow::bail!("model {index} has an empty model id");
            }
            if model.tiers.is_empty() {
                anyhow::bail!("model {} has no price tiers", model.model);
            }
            parse_timestamp(&model.effective_from)
                .with_context(|| format!("invalid effective_from for model {}", model.model))?;
            if model.aliases.iter().any(|alias| alias.trim().is_empty()) {
                anyhow::bail!("model {} has an empty alias", model.model);
            }
            for tier in &model.tiers {
                if tier.name.trim().is_empty() {
                    anyhow::bail!("model {} has an unnamed tier", model.model);
                }
                validate_rate(tier.input_per_mtok, "input", &model.model)?;
                validate_rate(tier.output_per_mtok, "output", &model.model)?;
                if let Some(rate) = tier.cache_read_per_mtok {
                    validate_rate(rate, "cache_read", &model.model)?;
                }
                for (ttl, rate) in &tier.cache_write_per_mtok {
                    if ttl.trim().is_empty() {
                        anyhow::bail!("model {} has an empty cache-write tier", model.model);
                    }
                    validate_rate(*rate, "cache_write", &model.model)?;
                }
                if !matches!(tier.reasoning_billing.as_str(), "output" | "none") {
                    anyhow::bail!(
                        "model {} has unsupported reasoning_billing {}",
                        model.model,
                        tier.reasoning_billing
                    );
                }
            }
            // Keep this variable in the validation path: a provider-looking
            // value in JSON must normalize successfully before use.
            let _ = provider;
        }
        Ok(())
    }

    /// Resolve a model alias for an event timestamp. Matching is exact for a
    /// canonical id/alias, or exact plus a dated snapshot suffix. Generic
    /// substring guesses are deliberately not accepted.
    pub fn resolve_model(
        &self,
        provider: Option<&str>,
        agent: Option<&str>,
        model: &str,
        timestamp: &str,
    ) -> std::result::Result<ResolvedModel, String> {
        let timestamp = parse_timestamp(timestamp).map_err(|e| e.to_string())?;
        let requested_provider = provider.and_then(normalize_provider);
        if provider.is_some() && requested_provider.is_none() {
            return Err(format!("unknown provider {}", provider.unwrap_or_default()));
        }
        let agent_provider = agent.and_then(infer_provider_from_agent);
        let model_provider = infer_provider_from_model(model);
        if let (Some(explicit), Some(inferred)) = (&requested_provider, &model_provider) {
            if explicit != inferred {
                return Err(format!(
                    "provider {} conflicts with model provider {}",
                    explicit, inferred
                ));
            }
        }
        if let (Some(explicit), Some(inferred)) = (&requested_provider, &agent_provider) {
            if explicit != inferred {
                return Err(format!(
                    "provider {} conflicts with agent provider {}",
                    explicit, inferred
                ));
            }
        }

        let model = normalize_model(model);
        if model.is_empty() {
            return Err("model is empty".to_string());
        }
        let provider = requested_provider.or(model_provider).or(agent_provider);
        let mut matches: Vec<&ModelPricing> = self
            .models
            .iter()
            .filter(|row| {
                provider
                    .as_deref()
                    .is_none_or(|p| normalize_provider(&row.provider).as_deref() == Some(p))
            })
            .filter(|row| model_matches(row, &model))
            .filter(|row| {
                parse_timestamp(&row.effective_from)
                    .map(|effective| effective <= timestamp)
                    .unwrap_or(false)
            })
            .collect();

        matches.sort_by(|left, right| {
            parse_timestamp(&right.effective_from)
                .ok()
                .cmp(&parse_timestamp(&left.effective_from).ok())
                .then_with(|| {
                    match_specificity(right, &model).cmp(&match_specificity(left, &model))
                })
        });
        let row = matches.into_iter().next().ok_or_else(|| {
            format!(
                "no effective pricing row for model {} at {}",
                model, timestamp
            )
        })?;
        Ok(ResolvedModel {
            provider: normalize_provider(&row.provider).unwrap_or_else(|| row.provider.clone()),
            model: row.model.clone(),
            effective_from: row.effective_from.clone(),
            pricing: row.clone(),
        })
    }

    fn tier_for_input(pricing: &ModelPricing, input_tokens: u64) -> Option<&PriceTier> {
        pricing
            .tiers
            .iter()
            .filter(|tier| tier.min_input_tokens.unwrap_or(0) <= input_tokens)
            .max_by_key(|tier| tier.min_input_tokens.unwrap_or(0))
    }
}

fn validate_rate(rate: f64, label: &str, model: &str) -> Result<()> {
    if !rate.is_finite() || rate < 0.0 {
        anyhow::bail!("model {model} has invalid {label} rate {rate}");
    }
    Ok(())
}

fn parse_timestamp(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(value).context("timestamp must be RFC3339")
}

fn normalize_provider(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase().replace('-', "_");
    match value.as_str() {
        "anthropic" | "claude" => Some("anthropic".to_string()),
        "openai" | "codex" => Some("openai".to_string()),
        "google" | "gemini" | "vertex" | "vertex_ai" => Some("google".to_string()),
        _ => None,
    }
}

fn infer_provider_from_agent(agent: &str) -> Option<String> {
    let agent = agent.to_ascii_lowercase();
    if agent.contains("anthropic") || agent.contains("claude") {
        Some("anthropic".to_string())
    } else if agent.contains("openai") || agent.contains("codex") {
        Some("openai".to_string())
    } else if agent.contains("google") || agent.contains("gemini") {
        Some("google".to_string())
    } else {
        None
    }
}

fn infer_provider_from_model(model: &str) -> Option<String> {
    let model = normalize_model(model);
    if model.starts_with("claude-") {
        Some("anthropic".to_string())
    } else if model.starts_with("gpt-")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
    {
        Some("openai".to_string())
    } else if model.starts_with("gemini-") {
        Some("google".to_string())
    } else {
        None
    }
}

fn normalize_model(value: &str) -> String {
    let mut value = value.trim().to_ascii_lowercase();
    for prefix in [
        "models/",
        "model/",
        "anthropic/",
        "openai/",
        "google/",
        "gemini/",
        "vertex_ai/",
    ] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.to_string();
        }
    }
    if let Some(stripped) = value.strip_suffix(":latest") {
        value = stripped.to_string();
    }
    value = value.replace(['_', '.'], "-");
    while value.contains("--") {
        value = value.replace("--", "-");
    }
    value.trim_matches('-').to_string()
}

fn model_matches(row: &ModelPricing, requested: &str) -> bool {
    std::iter::once(row.model.as_str())
        .chain(row.aliases.iter().map(String::as_str))
        .map(normalize_model)
        .any(|candidate| {
            candidate == requested
                || requested
                    .strip_prefix(&(candidate.clone() + "-"))
                    .is_some_and(is_snapshot_suffix)
        })
}

fn match_specificity(row: &ModelPricing, requested: &str) -> usize {
    std::iter::once(row.model.as_str())
        .chain(row.aliases.iter().map(String::as_str))
        .map(normalize_model)
        .filter(|candidate| {
            candidate == requested
                || requested
                    .strip_prefix(&(candidate.clone() + "-"))
                    .is_some_and(is_snapshot_suffix)
        })
        .map(|candidate| candidate.len())
        .max()
        .unwrap_or_default()
}

fn is_snapshot_suffix(suffix: &str) -> bool {
    let compact = suffix.replace('-', "");
    (compact.len() >= 8 && compact.chars().all(|c| c.is_ascii_digit()))
        || (suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_digit() || c == '-'))
}

/// Reported/computed provenance of the number exposed as `cost_usd`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CostSource {
    Reported,
    Computed,
    Unknown,
}

impl CostSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reported => "reported",
            Self::Computed => "computed",
            Self::Unknown => "unknown",
        }
    }
}

/// Result of comparing a provider-reported amount with Trace's calculation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    Match,
    Mismatch,
    ReportedOnly,
    ComputedOnly,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CostReconciliation {
    pub status: ReconciliationStatus,
    pub delta_usd: Option<f64>,
}

/// Additive pricing input seam for `usage_events`.
///
/// This mirrors the canonical `trace_core::usage::UsageEvent` fields without
/// coupling this module to the importer/storage implementation. The adapter
/// should pass the event's `agent`, optional provider, model, token fields,
/// RFC3339 `ts`, and only pass `cost_usd` as `reported_cost_usd` when
/// `cost_source == "reported"`. `input_tokens` is uncached input; cache read
/// and cache write tokens are additional billable token classes, matching the
/// provider usage APIs. `cache_write_tokens_by_ttl` is optional additive detail
/// for providers such as Anthropic that publish multiple write TTLs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageEventCostInput {
    pub provider: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    pub cache_write_tokens_by_ttl: BTreeMap<String, i64>,
    pub cache_write_ttl: Option<String>,
    pub reasoning_tokens: Option<i64>,
    pub ts: String,
    pub reported_cost_usd: Option<f64>,
}

/// Cost and provenance for one usage event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageEventCost {
    pub cost_usd: Option<f64>,
    pub reported_cost_usd: Option<f64>,
    pub computed_cost_usd: Option<f64>,
    pub cost_source: CostSource,
    pub pricing_version: Option<String>,
    pub pricing_model: Option<String>,
    pub reconciliation: CostReconciliation,
    pub warning: Option<String>,
}

/// Compute an event with the active local catalog.
pub fn compute_usage_event_cost(input: &UsageEventCostInput) -> UsageEventCost {
    let active = load_active_catalog_info().unwrap_or_else(|_| ActivePricing {
        catalog: embedded_catalog(),
        origin: PricingOrigin::Embedded,
        warning: Some("could not load local pricing override; using embedded catalog".to_string()),
    });
    let mut result = compute_usage_event_cost_with_catalog(&active.catalog, input);
    if let Some(active_warning) = active.warning {
        result.warning = Some(match result.warning {
            Some(existing) => format!("{active_warning}; {existing}"),
            None => active_warning,
        });
    }
    result
}

/// Compute an event against an explicit catalog. This is the deterministic
/// function used by tests and by callers that need to pin a release version.
pub fn compute_usage_event_cost_with_catalog(
    catalog: &PricingCatalog,
    input: &UsageEventCostInput,
) -> UsageEventCost {
    let valid_reported = match input.reported_cost_usd {
        Some(value) if value.is_finite() && value >= 0.0 => Some(value),
        Some(_) => None,
        None => None,
    };
    let invalid_reported = input.reported_cost_usd.is_some() && valid_reported.is_none();
    let mut warning = if invalid_reported {
        Some("reported cost is negative or non-finite and was ignored".to_string())
    } else {
        None
    };

    let calculation = calculate_cost(catalog, input);
    let (computed, pricing_model, calculation_warning) = match calculation {
        Ok(result) => (Some(result.0), Some(result.1), None),
        Err(reason) => (None, None, Some(reason)),
    };
    if let Some(reason) = calculation_warning {
        warning = Some(match warning {
            Some(existing) => format!("{existing}; {reason}"),
            None => reason,
        });
    }

    let reconciliation = match (valid_reported, computed) {
        (Some(reported), Some(computed)) => {
            let delta = reported - computed;
            CostReconciliation {
                status: if delta.abs() <= RECONCILIATION_TOLERANCE_USD {
                    ReconciliationStatus::Match
                } else {
                    ReconciliationStatus::Mismatch
                },
                delta_usd: Some(delta),
            }
        }
        (Some(_), None) => CostReconciliation {
            status: ReconciliationStatus::ReportedOnly,
            delta_usd: None,
        },
        (None, Some(_)) => CostReconciliation {
            status: ReconciliationStatus::ComputedOnly,
            delta_usd: None,
        },
        (None, None) => CostReconciliation {
            status: ReconciliationStatus::Unavailable,
            delta_usd: None,
        },
    };

    let (cost_usd, cost_source, pricing_version) = match (valid_reported, computed) {
        (Some(reported), Some(_)) | (Some(reported), None) => (
            Some(reported),
            CostSource::Reported,
            if computed.is_some() {
                Some(catalog.pricing_version.clone())
            } else {
                None
            },
        ),
        (None, Some(computed)) => (
            Some(computed),
            CostSource::Computed,
            Some(catalog.pricing_version.clone()),
        ),
        (None, None) => (None, CostSource::Unknown, None),
    };

    UsageEventCost {
        cost_usd,
        reported_cost_usd: valid_reported,
        computed_cost_usd: computed,
        cost_source,
        pricing_version,
        pricing_model,
        reconciliation,
        warning,
    }
}

fn calculate_cost(
    catalog: &PricingCatalog,
    input: &UsageEventCostInput,
) -> std::result::Result<(f64, String), String> {
    if input.provider.as_deref().is_some_and(is_local_provider) {
        return Ok((
            0.0,
            input.model.clone().unwrap_or_else(|| "local".to_string()),
        ));
    }
    let model = input
        .model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .ok_or_else(|| "model is missing; computed cost is unavailable".to_string())?;
    let input_tokens = nonnegative_tokens(input.input_tokens, "input_tokens")?;
    let output_tokens = nonnegative_tokens(input.output_tokens, "output_tokens")?;
    let cache_read_tokens =
        optional_nonnegative_tokens(input.cache_read_tokens, "cache_read_tokens")?;
    let cache_write_tokens =
        optional_nonnegative_tokens(input.cache_write_tokens, "cache_write_tokens")?;
    let reasoning_tokens = optional_nonnegative_tokens(input.reasoning_tokens, "reasoning_tokens")?;
    if reasoning_tokens > output_tokens {
        return Err("reasoning_tokens exceeds output_tokens".to_string());
    }
    let cache_write_tokens_for_context = if input.cache_write_tokens_by_ttl.is_empty() {
        cache_write_tokens
    } else {
        input
            .cache_write_tokens_by_ttl
            .values()
            .try_fold(0_u64, |total, tokens| {
                let tokens = nonnegative_tokens(Some(*tokens), "cache_write_tokens_by_ttl")?;
                total
                    .checked_add(tokens)
                    .ok_or_else(|| "cache-write token count overflow".to_string())
            })?
    };
    let prompt_tokens = input_tokens
        .checked_add(cache_read_tokens)
        .and_then(|total| total.checked_add(cache_write_tokens_for_context))
        .ok_or_else(|| "input token count overflow".to_string())?;

    let resolved = catalog
        .resolve_model(
            input.provider.as_deref(),
            input.agent.as_deref(),
            model,
            &input.ts,
        )
        .map_err(|reason| format!("{reason}; computed cost is unavailable"))?;
    let tier =
        PricingCatalog::tier_for_input(&resolved.pricing, prompt_tokens).ok_or_else(|| {
            format!(
                "no input tier for model {}; computed cost is unavailable",
                resolved.model
            )
        })?;

    let cache_read_cost = if cache_read_tokens > 0 {
        let rate = tier.cache_read_per_mtok.ok_or_else(|| {
            format!(
                "model {} has no published cache-read rate; computed cost is unavailable",
                resolved.model
            )
        })?;
        per_mtok(cache_read_tokens, rate)
    } else {
        0.0
    };

    let mut cache_write_cost = 0.0;
    if !input.cache_write_tokens_by_ttl.is_empty() {
        let mut aggregate = 0_u64;
        for (ttl, tokens) in &input.cache_write_tokens_by_ttl {
            let tokens = nonnegative_tokens(Some(*tokens), "cache_write_tokens_by_ttl")?;
            aggregate = aggregate
                .checked_add(tokens)
                .ok_or_else(|| "cache-write token count overflow".to_string())?;
            let rate = tier.cache_write_rate(Some(ttl)).ok_or_else(|| {
                format!(
                    "model {} has no published cache-write rate for TTL {}; computed cost is unavailable",
                    resolved.model, ttl
                )
            })?;
            cache_write_cost += per_mtok(tokens, rate);
        }
        if input.cache_write_tokens.is_some() && cache_write_tokens != aggregate {
            return Err(
                "cache_write_tokens disagrees with cache_write_tokens_by_ttl; computed cost is unavailable"
                    .to_string(),
            );
        }
    } else if cache_write_tokens > 0 {
        let rate = tier
            .cache_write_rate(input.cache_write_ttl.as_deref())
            .ok_or_else(|| {
                format!(
                    "model {} needs a supported cache-write tier/TTL; computed cost is unavailable",
                    resolved.model
                )
            })?;
        cache_write_cost = per_mtok(cache_write_tokens, rate);
    }

    let cost = per_mtok(input_tokens, tier.input_per_mtok)
        + cache_read_cost
        + cache_write_cost
        + per_mtok(output_tokens, tier.output_per_mtok);
    if !cost.is_finite() || cost < 0.0 {
        return Err("computed cost is non-finite; computed cost is unavailable".to_string());
    }
    Ok((cost, resolved.model))
}

fn nonnegative_tokens(value: Option<i64>, label: &str) -> std::result::Result<u64, String> {
    let value = value.ok_or_else(|| format!("{label} is missing; computed cost is unavailable"))?;
    if value < 0 {
        return Err(format!("{label} is negative; computed cost is unavailable"));
    }
    u64::try_from(value)
        .map_err(|_| format!("{label} is out of range; computed cost is unavailable"))
}

fn optional_nonnegative_tokens(
    value: Option<i64>,
    label: &str,
) -> std::result::Result<u64, String> {
    match value {
        Some(value) => nonnegative_tokens(Some(value), label),
        None => Ok(0),
    }
}

fn per_mtok(tokens: u64, rate: f64) -> f64 {
    tokens as f64 / 1_000_000.0 * rate
}

/// Backwards-compatible price shape for older Trace API callers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cached_input_per_mtok: f64,
}

/// Resolve the current catalog row for legacy callers.
pub fn price_for(model: &str) -> Option<ModelPrice> {
    let catalog = embedded_catalog();
    let row = catalog
        .resolve_model(None, None, model, &catalog.effective_from)
        .ok()?;
    let tier = PricingCatalog::tier_for_input(&row.pricing, 0)?;
    Some(ModelPrice {
        input_per_mtok: tier.input_per_mtok,
        output_per_mtok: tier.output_per_mtok,
        cached_input_per_mtok: tier.cache_read_per_mtok.unwrap_or(tier.input_per_mtok),
    })
}

/// Local models are explicitly unmetered. This is the only intentional zero
/// policy; an unknown hosted model never becomes zero.
pub fn is_local_provider(provider: &str) -> bool {
    let provider = provider.to_ascii_lowercase();
    matches!(
        provider.as_str(),
        "local" | "ollama" | "llamacpp" | "lmstudio"
    )
}

/// Legacy estimate API. Its `cached_tokens` parameter is a subset of
/// `input_tokens`; invalid/negative/excess values are unavailable rather than
/// clamped. New integrations should use [`UsageEventCostInput`], where cache
/// token classes are separate provider-reported counts.
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
    let input = nonnegative_tokens(input_tokens, "input_tokens").ok()?;
    let output = nonnegative_tokens(output_tokens, "output_tokens").ok()?;
    let cached = optional_nonnegative_tokens(cached_tokens, "cached_tokens").ok()?;
    if cached > input {
        return None;
    }
    Some(
        per_mtok(input - cached, price.input_per_mtok)
            + per_mtok(cached, price.cached_input_per_mtok)
            + per_mtok(output, price.output_per_mtok),
    )
}

/// Where the active catalog came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PricingOrigin {
    Embedded,
    LocalOverride,
}

impl PricingOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::LocalOverride => "local override",
        }
    }
}

/// Active catalog plus its source and a non-fatal fallback warning.
#[derive(Debug, Clone)]
pub struct ActivePricing {
    pub catalog: PricingCatalog,
    pub origin: PricingOrigin,
    pub warning: Option<String>,
}

/// Parse the pinned catalog compiled into Trace.
pub fn embedded_catalog() -> PricingCatalog {
    PricingCatalog::from_json(EMBEDDED_PRICING_JSON).expect("embedded pricing catalog is valid")
}

/// Load a valid `TRACE_HOME/pricing.json` override, or use the pinned catalog.
pub fn load_active_catalog_info() -> Result<ActivePricing> {
    let embedded = embedded_catalog();
    let path = paths::pricing_path()?;
    if !path.exists() {
        return Ok(ActivePricing {
            catalog: embedded,
            origin: PricingOrigin::Embedded,
            warning: None,
        });
    }
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) => {
            return Ok(ActivePricing {
                catalog: embedded,
                origin: PricingOrigin::Embedded,
                warning: Some(format!(
                    "ignored unreadable pricing override {}: {error}",
                    path.display()
                )),
            });
        }
    };
    match PricingCatalog::from_json(&json) {
        Ok(catalog) => Ok(ActivePricing {
            catalog,
            origin: PricingOrigin::LocalOverride,
            warning: None,
        }),
        Err(error) => Ok(ActivePricing {
            catalog: embedded,
            origin: PricingOrigin::Embedded,
            warning: Some(format!(
                "ignored invalid pricing override {}: {error}",
                path.display()
            )),
        }),
    }
}

/// The active pricing version shown by `trc --version` and `trc doctor`.
pub fn pricing_version() -> String {
    load_active_catalog_info()
        .map(|active| active.catalog.pricing_version)
        .unwrap_or_else(|_| embedded_catalog().pricing_version)
}

/// Result of `trc pricing update`.
#[derive(Debug, Clone)]
pub struct PricingUpdateOutcome {
    pub catalog: PricingCatalog,
    pub path: PathBuf,
    pub used_remote: bool,
    pub warning: Option<String>,
}

/// Fetch, structurally validate, and persist a LiteLLM-derived local catalog.
/// Any network, parse, validation, or write failure returns the pinned catalog
/// with a warning so an offline update never destroys a working installation.
pub fn update_pricing_from_litellm() -> Result<PricingUpdateOutcome> {
    let path = paths::pricing_path()?;
    let embedded = embedded_catalog();
    let remote: Result<PricingCatalog> = (|| {
        let response = ureq::get(LITELLM_PRICE_URL)
            .call()
            .map_err(|error| anyhow!("fetching LiteLLM pricing: {error}"))?;
        let body = response
            .into_string()
            .map_err(|error| anyhow!("reading LiteLLM pricing response: {error}"))?;
        parse_litellm_catalog(&body)
    })();
    let remote = match remote {
        Ok(catalog) => catalog,
        Err(error) => {
            return Ok(PricingUpdateOutcome {
                catalog: embedded,
                path,
                used_remote: false,
                warning: Some(format!("pricing update used embedded catalog: {error}")),
            });
        }
    };
    let json = serde_json::to_vec_pretty(&remote).context("serializing updated pricing catalog")?;
    if let Err(error) = paths::ensure_global_dir()
        .and_then(|_| std::fs::write(&path, json).context("writing pricing override"))
    {
        return Ok(PricingUpdateOutcome {
            catalog: embedded,
            path,
            used_remote: false,
            warning: Some(format!(
                "pricing update could not persist remote catalog: {error}"
            )),
        });
    }
    Ok(PricingUpdateOutcome {
        catalog: remote,
        path,
        used_remote: true,
        warning: None,
    })
}

/// Convert LiteLLM's model-price object into Trace's validated catalog. This
/// function is public for deterministic update tests; the remote source itself
/// is not treated as authoritative provider evidence.
pub fn parse_litellm_catalog(json: &str) -> Result<PricingCatalog> {
    let value: Value = serde_json::from_str(json).context("invalid LiteLLM JSON")?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("LiteLLM price file is not an object"))?;
    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let effective_from = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut models = Vec::new();
    let mut providers = BTreeMap::<String, bool>::new();
    for (model_key, value) in object {
        let Some(record) = value.as_object() else {
            continue;
        };
        let Some(provider) = record
            .get("litellm_provider")
            .and_then(Value::as_str)
            .and_then(normalize_provider_from_litellm)
        else {
            continue;
        };
        let Some(input) = number(record, "input_cost_per_token") else {
            continue;
        };
        let Some(output) = number(record, "output_cost_per_token") else {
            continue;
        };
        if !input.is_finite() || !output.is_finite() || input < 0.0 || output < 0.0 {
            continue;
        }
        let mut cache_write = BTreeMap::new();
        if let Some(rate) = number(record, "cache_creation_input_token_cost")
            .filter(|rate| rate.is_finite() && *rate >= 0.0)
        {
            cache_write.insert("default".to_string(), rate * 1_000_000.0);
        }
        let cache_read = number(record, "cache_read_input_token_cost")
            .filter(|rate| rate.is_finite() && *rate >= 0.0)
            .map(|rate| rate * 1_000_000.0);
        let reasoning = if record
            .get("supports_reasoning")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || record
                .get("supports_reasoning_content")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            "output"
        } else {
            "none"
        };
        let aliases = record
            .get("aliases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
        models.push(ModelPricing {
            provider: provider.clone(),
            model: model_key.clone(),
            aliases,
            effective_from: effective_from.clone(),
            tiers: vec![PriceTier {
                name: "standard".to_string(),
                min_input_tokens: None,
                input_per_mtok: input * 1_000_000.0,
                output_per_mtok: output * 1_000_000.0,
                cache_read_per_mtok: cache_read,
                cache_write_per_mtok: cache_write,
                reasoning_billing: reasoning.to_string(),
                notes: Some("Converted from LiteLLM; verify against the provider page before relying on a new row.".to_string()),
            }],
        });
        providers.insert(provider, true);
    }
    for provider in ["anthropic", "openai", "google"] {
        if !providers.contains_key(provider) {
            anyhow::bail!("LiteLLM catalog has no usable {provider} rows");
        }
    }
    if models.is_empty() {
        anyhow::bail!("LiteLLM catalog has no usable provider rows");
    }
    let catalog = PricingCatalog {
        description: Some("Trace pricing update converted from LiteLLM; official provider citations remain in the embedded catalog.".to_string()),
        schema_version: 1,
        pricing_version: format!("litellm-{date}"),
        effective_from,
        sources: vec![PricingSource {
            provider: "community".to_string(),
            url: LITELLM_PRICE_URL.to_string(),
            retrieved_at: date,
            notes: "Structural update source only; verify rates against the provider's official pricing page before release.".to_string(),
        }],
        models,
    };
    catalog.validate()?;
    Ok(catalog)
}

fn number(object: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}

fn normalize_provider_from_litellm(value: &str) -> Option<String> {
    let value = value.to_ascii_lowercase();
    if value.contains("anthropic") {
        Some("anthropic".to_string())
    } else if value.contains("openai") {
        Some("openai".to_string())
    } else if value.contains("gemini") || value.contains("google") || value.contains("vertex") {
        Some("google".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-10-12T12:00:00Z";

    fn input(model: &str, input_tokens: i64, output_tokens: i64) -> UsageEventCostInput {
        UsageEventCostInput {
            provider: None,
            agent: None,
            model: Some(model.to_string()),
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            cache_read_tokens: Some(0),
            cache_write_tokens: Some(0),
            ts: NOW.to_string(),
            ..UsageEventCostInput::default()
        }
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("expected a cost");
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn embedded_catalog_is_valid_and_cited() {
        let catalog = embedded_catalog();
        assert_eq!(catalog.schema_version, 1);
        assert!(catalog.pricing_version.starts_with("official-"));
        assert!(catalog
            .sources
            .iter()
            .any(|source| source.provider == "anthropic"));
        assert!(catalog
            .sources
            .iter()
            .any(|source| source.provider == "openai"));
        assert!(catalog
            .sources
            .iter()
            .any(|source| source.provider == "google"));
    }

    #[test]
    fn anthropic_opus_standard_cache_write_five_minute_is_hand_calculated() {
        let mut event = input("claude-opus-4.6", 4_000_000, 1_000_000);
        event.cache_read_tokens = Some(1_000_000);
        event.cache_write_tokens = Some(1_000_000);
        event.cache_write_ttl = Some("ephemeral_5m".to_string());
        // uncached input: 4M * $5 + cache read: 1M * $0.50 + 5m write: 1M * $6.25 + output: 1M * $25 = $51.75
        assert_close(compute_usage_event_cost(&event).computed_cost_usd, 51.75);
    }

    #[test]
    fn anthropic_opus_one_hour_write_is_hand_calculated() {
        let mut event = input("anthropic/claude-opus-4-6", 4_000_000, 1_000_000);
        event.cache_write_tokens = Some(1_000_000);
        event.cache_write_ttl = Some("1h".to_string());
        // 4M * $5 + 1M * $10 + 1M * $25 = $55.00.
        assert_close(compute_usage_event_cost(&event).computed_cost_usd, 55.0);
    }

    #[test]
    fn anthropic_cache_write_ttls_are_aggregated_and_hand_calculated() {
        let mut event = input("claude-sonnet-4-6", 100, 20);
        event.cache_read_tokens = Some(30);
        event.cache_write_tokens = Some(40);
        event.cache_write_tokens_by_ttl.insert("5m".to_string(), 25);
        event.cache_write_tokens_by_ttl.insert("1h".to_string(), 15);
        // 100*$3/M + 30*$.30/M + 25*$3.75/M + 15*$6/M + 20*$15/M = $.00079275.
        assert_close(
            compute_usage_event_cost(&event).computed_cost_usd,
            0.00079275,
        );

        event.cache_write_tokens = Some(39);
        let mismatch = compute_usage_event_cost(&event);
        assert_eq!(mismatch.cost_source, CostSource::Unknown);
        assert!(mismatch.warning.unwrap().contains("disagrees"));
    }

    #[test]
    fn anthropic_sonnet_and_haiku_rates_are_hand_calculated() {
        // Sonnet: 1M * $3 + 1M * $15 = $18.00.
        assert_close(
            compute_usage_event_cost(&input("claude-sonnet-4", 1_000_000, 1_000_000))
                .computed_cost_usd,
            18.0,
        );

        let mut haiku = input("claude-haiku-5-5", 2_000_000, 1_000_000);
        haiku.cache_read_tokens = Some(500_000);
        haiku.cache_write_tokens = Some(100_000);
        haiku.cache_write_ttl = Some("5m".to_string());
        // 2M * $.50 + .5M * $.05 + .1M * $.625 + 1M * $2.50 = $3.5875.
        assert_close(compute_usage_event_cost(&haiku).computed_cost_usd, 3.5875);
    }

    #[test]
    fn anthropic_haiku_long_context_selects_the_long_tier() {
        let event = input("claude-haiku-5.5", 100_001, 1_000_000);
        // 100001 * $.50/M + 1M * $2.50 = $2.5500005.
        assert_close(
            compute_usage_event_cost(&event).computed_cost_usd,
            2.5500005,
        );
    }

    #[test]
    fn openai_gpt4o_mini_and_gpt4o_cache_are_hand_calculated() {
        let mini = input("gpt-4o-mini", 1_234, 567);
        // 1234 * $.15/M + 567 * $.60/M = $.0005253.
        assert_close(compute_usage_event_cost(&mini).computed_cost_usd, 0.0005253);

        let mut gpt4o = input("gpt-4o", 1_000_000, 1_000_000);
        gpt4o.cache_read_tokens = Some(400_000);
        // 1M * $2.50 + .4M * $1.25 + 1M * $10 = $13.00.
        assert_close(compute_usage_event_cost(&gpt4o).computed_cost_usd, 13.0);
    }

    #[test]
    fn openai_gpt54_short_and_long_context_are_hand_calculated() {
        let mut short = input("openai/gpt-5.4", 100_000, 1_000_000);
        short.cache_read_tokens = Some(50_000);
        short.cache_write_tokens = Some(25_000);
        // .1M * $2.50 + .05M * $.25 + .025M * $2.50 + 1M * $15 = $15.325.
        assert_close(compute_usage_event_cost(&short).computed_cost_usd, 15.325);

        let long = input("gpt-5.4", 272_001, 1_000_000);
        // 272001 * $5/M + 1M * $22.50 = $23.860005.
        assert_close(compute_usage_event_cost(&long).computed_cost_usd, 23.860005);

        let gpt56 = input("gpt-5.6", 272_001, 1_000_000);
        // 272001 * $8/M + 1M * $30 = $32.176008.
        assert_close(
            compute_usage_event_cost(&gpt56).computed_cost_usd,
            32.176008,
        );

        let mut gpt61 = input("gpt-6.1-sol", 100_000, 1_000_000);
        gpt61.cache_read_tokens = Some(50_000);
        gpt61.cache_write_tokens = Some(25_000);
        // .1M*$2 + .05M*$.10 + .025M*$2.50 + 1M*$10 = $10.2675.
        assert_close(compute_usage_event_cost(&gpt61).computed_cost_usd, 10.2675);
    }

    #[test]
    fn google_rates_and_long_context_are_hand_calculated() {
        let mut pro = input("models/gemini-3.1-pro-preview", 100_000, 1_000_000);
        pro.cache_read_tokens = Some(50_000);
        // .1M * $2 + .05M * $.20 + 1M * $12 = $12.21.
        assert_close(compute_usage_event_cost(&pro).computed_cost_usd, 12.21);

        let mut pro_long = input("gemini-3.1-pro-preview", 1_000_000, 1_000_000);
        pro_long.cache_read_tokens = Some(200_000);
        // 1M * $4 + .2M * $.40 + 1M * $18 = $22.08.
        assert_close(compute_usage_event_cost(&pro_long).computed_cost_usd, 22.08);

        let long = input("gemini-3.1-pro-preview", 200_001, 1_000_000);
        // 200001 * $4/M + 1M * $18 = $18.800004.
        assert_close(compute_usage_event_cost(&long).computed_cost_usd, 18.800004);

        let flash = input("gemini-3.5-flash-lite", 1_000_000, 1_000_000);
        // 1M * $.30 + 1M * $2.50 = $2.80.
        assert_close(compute_usage_event_cost(&flash).computed_cost_usd, 2.8);
    }

    #[test]
    fn reasoning_tokens_are_inclusive_and_not_double_charged() {
        let mut event = input("gpt-5", 1_000_000, 1_000_000);
        event.reasoning_tokens = Some(400_000);
        // Reasoning is a detail of the 1M output token count: 1.25 + 10 = $11.25.
        assert_close(compute_usage_event_cost(&event).computed_cost_usd, 11.25);
    }

    #[test]
    fn google_cache_write_is_unknown_because_only_storage_is_published() {
        let mut event = input("gemini-3-flash-preview", 1_000, 1_000);
        event.cache_write_tokens = Some(1);
        let result = compute_usage_event_cost(&event);
        assert_eq!(result.cost_source, CostSource::Unknown);
        assert!(result.cost_usd.is_none());
        assert!(result.warning.unwrap().contains("cache-write"));
    }

    #[test]
    fn anthropic_cache_write_without_ttl_is_unknown_when_multiple_tiers_exist() {
        let mut event = input("claude-sonnet-4-6", 100, 100);
        event.cache_write_tokens = Some(10);
        let result = compute_usage_event_cost(&event);
        assert_eq!(result.cost_source, CostSource::Unknown);
        assert!(result.warning.unwrap().contains("TTL"));
    }

    #[test]
    fn aliases_provider_inference_and_dated_snapshots_resolve() {
        let mut anthropic = input("claude-sonnet-4-6-2026-01-01", 1_000_000, 0);
        anthropic.agent = Some("claude-code".to_string());
        assert_eq!(
            compute_usage_event_cost(&anthropic)
                .pricing_model
                .as_deref(),
            Some("claude-sonnet-4.6")
        );

        let openai = input("gpt-4o-mini-2024-07-18", 1_000_000, 0);
        assert_eq!(
            compute_usage_event_cost(&openai).pricing_model.as_deref(),
            Some("gpt-4o-mini")
        );

        let openai_alias = input("gpt-daybreak-blue-latest", 1, 0);
        assert_eq!(
            compute_usage_event_cost(&openai_alias)
                .pricing_model
                .as_deref(),
            Some("gpt-5.6-sol")
        );
    }

    #[test]
    fn timestamp_selects_the_latest_effective_snapshot() {
        let mut catalog = embedded_catalog();
        let mut newer = catalog
            .models
            .iter()
            .find(|row| row.model == "gpt-5")
            .expect("embedded gpt-5 row")
            .clone();
        newer.effective_from = "2026-10-13T00:00:00Z".to_string();
        newer.tiers[0].input_per_mtok = 9.0;
        catalog.models.push(newer);

        let before = input("gpt-5", 1_000_000, 0);
        assert_close(
            compute_usage_event_cost_with_catalog(&catalog, &before).computed_cost_usd,
            1.25,
        );

        let mut after = before;
        after.ts = "2026-10-13T00:00:00Z".to_string();
        assert_close(
            compute_usage_event_cost_with_catalog(&catalog, &after).computed_cost_usd,
            9.0,
        );
    }

    #[test]
    fn provider_conflict_is_unknown_not_a_guess() {
        let mut event = input("gpt-4o", 1, 1);
        event.provider = Some("anthropic".to_string());
        let result = compute_usage_event_cost(&event);
        assert_eq!(result.cost_source, CostSource::Unknown);
        assert!(result.cost_usd.is_none());
    }

    #[test]
    fn unknown_model_missing_tokens_and_old_timestamp_are_unavailable() {
        let unknown = input("mystery-model", 1, 1);
        let result = compute_usage_event_cost(&unknown);
        assert_eq!(result.cost_source, CostSource::Unknown);
        assert!(result.cost_usd.is_none());

        let missing = UsageEventCostInput {
            model: Some("gpt-5".to_string()),
            input_tokens: Some(1),
            output_tokens: None,
            ts: NOW.to_string(),
            ..UsageEventCostInput::default()
        };
        assert_eq!(
            compute_usage_event_cost(&missing).cost_source,
            CostSource::Unknown
        );

        let mut old = input("gpt-5", 1, 1);
        old.ts = "2026-01-15T00:00:00Z".to_string();
        assert_eq!(
            compute_usage_event_cost(&old).cost_source,
            CostSource::Unknown
        );
    }

    #[test]
    fn reported_cost_is_preserved_and_reconciled() {
        let mut event = input("gpt-5", 1_000_000, 1_000_000);
        event.reported_cost_usd = Some(11.25);
        let match_result = compute_usage_event_cost(&event);
        assert_eq!(match_result.cost_source, CostSource::Reported);
        assert_eq!(
            match_result.reconciliation.status,
            ReconciliationStatus::Match
        );
        assert_eq!(
            match_result.pricing_version.as_deref(),
            Some("official-2026-10-12")
        );

        event.reported_cost_usd = Some(12.0);
        let mismatch = compute_usage_event_cost(&event);
        assert_eq!(mismatch.cost_usd, Some(12.0));
        assert_eq!(mismatch.computed_cost_usd, Some(11.25));
        assert_eq!(
            mismatch.reconciliation.status,
            ReconciliationStatus::Mismatch
        );
        assert_eq!(mismatch.reconciliation.delta_usd, Some(0.75));

        event.model = Some("mystery-model".to_string());
        event.reported_cost_usd = Some(0.25);
        let reported_only = compute_usage_event_cost(&event);
        assert_eq!(reported_only.cost_source, CostSource::Reported);
        assert_eq!(
            reported_only.reconciliation.status,
            ReconciliationStatus::ReportedOnly
        );
        assert!(reported_only.pricing_version.is_none());
    }

    #[test]
    fn invalid_reported_cost_is_not_used_as_zero() {
        let mut event = input("gpt-5", 1_000_000, 1_000_000);
        event.reported_cost_usd = Some(f64::NAN);
        let result = compute_usage_event_cost(&event);
        assert_eq!(result.cost_source, CostSource::Computed);
        assert_eq!(result.reported_cost_usd, None);
        assert!(result.warning.unwrap().contains("non-finite"));
    }

    #[test]
    fn legacy_estimate_rejects_invalid_cached_counts() {
        assert_eq!(
            estimate_cost("anthropic", "claude-sonnet-4", Some(10), Some(10), Some(11)),
            None
        );
        assert_eq!(
            estimate_cost("anthropic", "claude-sonnet-4", Some(10), Some(10), Some(-1)),
            None
        );
        assert_eq!(
            estimate_cost("anthropic", "claude-sonnet-4", Some(10), None, None),
            None
        );
        assert_eq!(
            estimate_cost("ollama", "mystery", None, None, None),
            Some(0.0)
        );
    }

    #[test]
    fn litellm_update_conversion_is_structural_and_requires_three_providers() {
        let json = r#"{
          "claude-test": {"litellm_provider":"anthropic", "input_cost_per_token":0.000003, "output_cost_per_token":0.000015, "cache_read_input_token_cost":0.0000003, "cache_creation_input_token_cost":0.000003, "supports_reasoning":true},
          "gpt-test": {"litellm_provider":"openai", "input_cost_per_token":0.000002, "output_cost_per_token":0.000008},
          "gemini-test": {"litellm_provider":"gemini", "input_cost_per_token":0.000001, "output_cost_per_token":0.000004}
        }"#;
        let catalog = parse_litellm_catalog(json).unwrap();
        assert!(catalog.pricing_version.starts_with("litellm-"));
        let row = catalog
            .models
            .iter()
            .find(|row| row.model == "claude-test")
            .unwrap();
        assert_eq!(row.tiers[0].input_per_mtok, 3.0);
        assert_eq!(row.tiers[0].cache_write_per_mtok.get("default"), Some(&3.0));
        assert_eq!(row.tiers[0].reasoning_billing, "output");
    }
}
