//! The dated price table and what a cost is (PRD §10.5): every `Cost` names the table it used.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::tokens::TokenCounts;
use crate::model::{Role, TokenUsage};

/// Per-million rates of one model, USD, as the table lists them.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Rates {
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub cache_read_per_m: f64,
    pub cache_write_per_m: f64,
}

const PER_MILLION: f64 = 1e6;

impl Rates {
    /// The three classes a usage-log turn carries; the log has no cache-write count.
    fn turn_usd(&self, tokens: &TokenCounts) -> f64 {
        (tokens.input as f64 * self.input_per_m
            + tokens.output as f64 * self.output_per_m
            + tokens.cache_read as f64 * self.cache_read_per_m)
            / PER_MILLION
    }

    /// The four classes of a sidecar report.
    fn usage_usd(&self, usage: &TokenUsage) -> f64 {
        (usage.input as f64 * self.input_per_m
            + usage.output as f64 * self.output_per_m
            + usage.cache_read as f64 * self.cache_read_per_m
            + usage.cache_write as f64 * self.cache_write_per_m)
            / PER_MILLION
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PriceError {
    #[error("{path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("the price table is not valid: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Deserialize)]
struct Raw {
    effective: String,
    #[serde(default = "usd")]
    currency: String,
    #[serde(default)]
    models: BTreeMap<String, Rates>,
    #[serde(default)]
    roles: BTreeMap<String, f64>,
}

fn usd() -> String {
    "USD".to_owned()
}

/// `config/prices/<date>.toml`: the rates per model, and the optional `[roles]` multiplier of
/// each role (1.0 when a role has none).
#[derive(Clone, Debug, PartialEq)]
pub struct PriceTable {
    effective: String,
    currency: String,
    models: BTreeMap<String, Rates>,
    roles: BTreeMap<String, f64>,
}

impl PriceTable {
    pub fn parse(text: &str) -> Result<Self, PriceError> {
        let raw: Raw = toml::from_str(text)?;
        Ok(Self {
            effective: raw.effective,
            currency: raw.currency,
            models: raw.models,
            roles: raw.roles,
        })
    }

    pub fn load(path: &Path) -> Result<Self, PriceError> {
        let text = std::fs::read_to_string(path).map_err(|source| PriceError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// The `effective` date every cost names.
    pub fn effective(&self) -> &str {
        &self.effective
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }

    pub fn rates(&self, model: &str) -> Option<&Rates> {
        self.models.get(model)
    }

    /// The key under which a role's multiplier is named in the table.
    pub fn multiplier_key(role: Role) -> String {
        format!("roles.{}", role.as_str())
    }

    pub fn multiplier(&self, role: Role) -> f64 {
        self.roles.get(role.as_str()).copied().unwrap_or(1.0)
    }

    /// USD of a turn: tokens x the model's rate x the role's multiplier; `None` when the table
    /// has no row for the model.
    pub fn turn_usd(&self, role: Role, model: Option<&str>, tokens: &TokenCounts) -> Option<f64> {
        let rates = self.rates(model?)?;
        Some(rates.turn_usd(tokens) * self.multiplier(role))
    }

    /// The cost of a sidecar report: its credits when the stream exposes them, else USD from the
    /// table; `None` when there are no credits and the model has no row.
    pub fn session_cost(&self, usage: &TokenUsage, model: Option<&str>) -> Option<Cost> {
        match usage.credits {
            Some(credits) => Some(self.cost(credits, CostUnit::Credits)),
            None => {
                let usd = self.rates(model?)?.usage_usd(usage);
                Some(self.cost(usd, CostUnit::Usd))
            }
        }
    }

    pub fn cost(&self, amount: f64, unit: CostUnit) -> Cost {
        Cost {
            amount,
            unit,
            price_table: self.effective.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostUnit {
    Usd,
    Credits,
}

/// An amount and the price table that produced it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Cost {
    pub amount: f64,
    pub unit: CostUnit,
    pub price_table: String,
}
