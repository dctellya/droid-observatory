//! The budget meter (PRD FR-M6): cost so far against the optional budget.

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BudgetMeter {
    /// False when no budget above zero is set: the Panel shows no fraction.
    pub enabled: bool,
    pub budget: Option<f64>,
    pub cost_so_far: f64,
    /// `cost_so_far / budget`, only when enabled.
    pub fraction: Option<f64>,
    pub currency: String,
    pub price_table: String,
}

/// `config_budget` is `[metrics].budget`; a launch `budget_usd` above zero wins over it. A
/// budget of zero or less disables the meter.
pub fn budget_meter(
    config_budget: f64,
    launch_budget_usd: Option<f64>,
    cost_so_far: f64,
    price_table: &str,
    currency: &str,
) -> BudgetMeter {
    let budget = launch_budget_usd
        .filter(|launched| *launched > 0.0)
        .unwrap_or(config_budget);
    let enabled = budget > 0.0;
    BudgetMeter {
        enabled,
        budget: enabled.then_some(budget),
        cost_so_far,
        fraction: enabled.then(|| cost_so_far / budget),
        currency: currency.to_owned(),
        price_table: price_table.to_owned(),
    }
}
