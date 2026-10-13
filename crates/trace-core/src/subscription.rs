//! Local, versioned monthly subscription list prices.
//!
//! Subscription prices are flat plan fees; they are not token prices or
//! per-session charges. Cursor's published included model-usage amount is kept
//! separately so it cannot be mistaken for the subscription fee.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const EMBEDDED_SUBSCRIPTION_JSON: &str = include_str!("../data/subscription-pricing.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionPricingSource {
    pub provider: String,
    pub url: String,
    pub retrieved_at: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionPlanRate {
    pub provider: String,
    pub plan_id: String,
    pub plan_name: String,
    pub monthly_price_usd: f64,
    #[serde(default)]
    pub included_api_usage_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionPricingCatalog {
    pub schema_version: u32,
    pub pricing_version: String,
    pub region: String,
    pub currency: String,
    pub effective_from: String,
    pub sources: Vec<SubscriptionPricingSource>,
    pub plans: Vec<SubscriptionPlanRate>,
}

impl SubscriptionPricingCatalog {
    pub fn from_json(json: &str) -> Result<Self> {
        let catalog: Self = serde_json::from_str(json)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<()> {
        chrono::DateTime::parse_from_rfc3339(&self.effective_from)
            .map_err(|error| anyhow!("invalid subscription pricing effective_from: {error}"))?;
        if self.schema_version != 1 {
            anyhow::bail!(
                "unsupported subscription pricing schema {}",
                self.schema_version
            );
        }
        if self.pricing_version.trim().is_empty()
            || self.region.trim().is_empty()
            || self.currency != "USD"
            || self.plans.is_empty()
        {
            anyhow::bail!("subscription pricing catalog is incomplete");
        }
        let mut keys = std::collections::HashSet::new();
        for source in &self.sources {
            if source.provider.trim().is_empty()
                || source.url.trim().is_empty()
                || source.retrieved_at.trim().is_empty()
            {
                anyhow::bail!("subscription pricing source is incomplete");
            }
        }
        for plan in &self.plans {
            if plan.provider.trim().is_empty()
                || plan.plan_id.trim().is_empty()
                || plan.plan_name.trim().is_empty()
                || !plan.monthly_price_usd.is_finite()
                || plan.monthly_price_usd < 0.0
                || plan
                    .included_api_usage_usd
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
                || !keys.insert((plan.provider.as_str(), plan.plan_id.as_str()))
            {
                anyhow::bail!("invalid or duplicate subscription plan rate");
            }
            if !self
                .sources
                .iter()
                .any(|source| source.provider == plan.provider)
            {
                return Err(anyhow!("subscription plan {} has no source", plan.plan_id));
            }
        }
        Ok(())
    }
}

pub fn embedded_catalog() -> SubscriptionPricingCatalog {
    SubscriptionPricingCatalog::from_json(EMBEDDED_SUBSCRIPTION_JSON)
        .expect("embedded subscription pricing catalog is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_subscription_rates_are_valid_and_source_backed() {
        let catalog = embedded_catalog();
        assert_eq!(catalog.region, "US");
        assert!(catalog
            .plans
            .iter()
            .any(|plan| plan.plan_id == "claude-pro"));
        assert!(catalog.plans.iter().any(|plan| {
            plan.plan_id == "cursor-pro-plus" && plan.included_api_usage_usd == Some(70.0)
        }));
    }
}
