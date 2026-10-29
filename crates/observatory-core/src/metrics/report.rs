//! The cost half of the metrics report: tokens and cost per role, lane and milestone, tokens in
//! waste, the sidecar reconciliation, overhead, the budget meter and the model comparison.

use std::collections::BTreeMap;

use serde::Serialize;

use super::budget::{BudgetMeter, budget_meter};
use super::cards::{CardTime, card_times};
use super::compare::{ModelComparisonRow, model_comparison};
use super::notes::Notes;
use super::price::{Cost, CostUnit, PriceTable};
use super::state::TimeState;
use super::throttled::throttled;
use super::tokens::{
    Reconciliation, TokenCounts, TokensInWaste, reconcile, tokens_in_waste, turns,
};
use super::waste::WasteCause;
use crate::config::OverheadApportionment;
use crate::model::{
    CycleTemplate, Event, EventKind, FeatureId, LaneId, MilestoneId, ModelId, Role, SessionId,
    Timestamp,
};

pub struct CostInput<'a> {
    pub events: &'a [Event],
    pub template: &'a CycleTemplate,
    pub notes: &'a Notes,
    pub prices: &'a PriceTable,
    /// The instant the report is made at: the mission's last event, or now while it runs.
    pub end: Timestamp,
    /// `[metrics].budget`.
    pub budget: f64,
    /// The launch form's `budget_usd`.
    pub launch_budget_usd: Option<f64>,
    pub apportionment: OverheadApportionment,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RoleTokens {
    pub role: Role,
    pub tokens: TokenCounts,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RoleCost {
    pub role: Role,
    pub cost: Cost,
    /// Where the table names the multiplier (`roles.worker`); 1.0 when it names none.
    pub multiplier_key: String,
    pub multiplier: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LaneUsage {
    pub lane: LaneId,
    pub tokens: TokenCounts,
    pub cost: Cost,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MilestoneUsage {
    pub milestone: Option<MilestoneId>,
    pub tokens: TokenCounts,
    pub cost: Cost,
}

/// A session's own report: credits when the stream exposes them, else the table's USD.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SessionCost {
    pub session: SessionId,
    pub role: Role,
    pub model: Option<ModelId>,
    pub cost: Cost,
}

/// Tokens of a model the price table has no row for: counted, not priced (rule 18).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnpricedTokens {
    pub model: Option<ModelId>,
    pub tokens: TokenCounts,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OverheadShare {
    pub feature: FeatureId,
    pub cost: Cost,
    pub active_s: f64,
}

/// What no feature owns: the orchestrator's own turns and work, and its waits. Separate by
/// default (PRD §10.3); `equal` splits it over the features and labels it `apportioned`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Overhead {
    pub label: String,
    pub tokens: TokenCounts,
    pub cost: Cost,
    pub active_s: f64,
    /// The mission level's own waits, in seconds per cause.
    pub waste_s: BTreeMap<WasteCause, u64>,
    pub per_feature: Vec<OverheadShare>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CostReport {
    pub price_table: String,
    pub currency: String,
    pub total_tokens: TokenCounts,
    pub cost_so_far: Cost,
    pub tokens_by_role: Vec<RoleTokens>,
    pub cost_by_role: Vec<RoleCost>,
    /// One row per lane of the template, in its order; a lane with no turn is a zero row.
    pub tokens_by_lane: Vec<LaneUsage>,
    /// Turns in a lane the template does not have, so the lane rows add up to the total.
    pub tokens_other_lanes: TokenCounts,
    pub tokens_by_milestone: Vec<MilestoneUsage>,
    pub tokens_in_waste_by_cause: TokensInWaste,
    pub session_costs: Vec<SessionCost>,
    pub reconciliation: Reconciliation,
    pub unpriced: Vec<UnpricedTokens>,
    pub budget: BudgetMeter,
    pub model_comparison: Vec<ModelComparisonRow>,
    pub overhead: Overhead,
}

impl CostReport {
    /// Every cost in the report, each of which names its price table.
    pub fn costs(&self) -> Vec<&Cost> {
        let mut all = vec![&self.cost_so_far, &self.overhead.cost];
        all.extend(self.cost_by_role.iter().map(|row| &row.cost));
        all.extend(self.tokens_by_lane.iter().map(|row| &row.cost));
        all.extend(self.tokens_by_milestone.iter().map(|row| &row.cost));
        all.extend(self.session_costs.iter().map(|row| &row.cost));
        all.extend(self.model_comparison.iter().map(|row| &row.cost));
        all.extend(self.overhead.per_feature.iter().map(|row| &row.cost));
        all
    }
}

#[derive(Default)]
struct Tally {
    tokens: TokenCounts,
    usd: f64,
}

impl Tally {
    fn add(&mut self, tokens: TokenCounts, usd: Option<f64>) {
        self.tokens += tokens;
        self.usd += usd.unwrap_or(0.0);
    }
}

pub fn cost_report(input: &CostInput<'_>) -> CostReport {
    let CostInput {
        events,
        template,
        notes,
        prices,
        end,
        ..
    } = *input;
    let stops = throttled(events, notes, end);
    let stop_spans = stops.spans();
    let cards = card_times(events, template, notes, end, &stop_spans);

    let mut total = Tally::default();
    let mut by_role: BTreeMap<usize, Tally> = BTreeMap::new();
    let mut by_lane: BTreeMap<LaneId, Tally> = BTreeMap::new();
    let mut by_milestone: BTreeMap<Option<MilestoneId>, Tally> = BTreeMap::new();
    let mut unpriced: BTreeMap<Option<ModelId>, TokenCounts> = BTreeMap::new();
    let mut mission_level = Tally::default();
    for turn in turns(events) {
        let model = turn.model.map(|m| m.0.as_str());
        let usd = prices.turn_usd(turn.event.role, model, &turn.tokens);
        if usd.is_none() {
            *unpriced.entry(turn.model.cloned()).or_default() += turn.tokens;
        }
        total.add(turn.tokens, usd);
        let role = Role::ALL
            .iter()
            .position(|r| *r == turn.event.role)
            .unwrap_or(usize::MAX);
        by_role.entry(role).or_default().add(turn.tokens, usd);
        by_lane
            .entry(turn.event.lane.clone())
            .or_default()
            .add(turn.tokens, usd);
        by_milestone
            .entry(turn.event.milestone_id.clone())
            .or_default()
            .add(turn.tokens, usd);
        if turn.event.feature_id.is_none() {
            mission_level.add(turn.tokens, usd);
        }
    }

    let usd = |tally: &Tally| prices.cost(tally.usd, CostUnit::Usd);
    let tokens_by_lane: Vec<LaneUsage> = template
        .lanes
        .iter()
        .map(|lane| {
            let tally = by_lane.remove(&lane.id).unwrap_or_default();
            LaneUsage {
                lane: lane.id.clone(),
                tokens: tally.tokens,
                cost: usd(&tally),
            }
        })
        .collect();
    let mut other_lanes = TokenCounts::default();
    for tally in by_lane.values() {
        other_lanes += tally.tokens;
    }

    CostReport {
        price_table: prices.effective().to_owned(),
        currency: prices.currency().to_owned(),
        total_tokens: total.tokens,
        cost_so_far: usd(&total),
        tokens_by_role: by_role
            .iter()
            .filter_map(|(rank, tally)| {
                Some(RoleTokens {
                    role: *Role::ALL.get(*rank)?,
                    tokens: tally.tokens,
                })
            })
            .collect(),
        cost_by_role: by_role
            .iter()
            .filter_map(|(rank, tally)| {
                let role = *Role::ALL.get(*rank)?;
                Some(RoleCost {
                    role,
                    cost: usd(tally),
                    multiplier_key: PriceTable::multiplier_key(role),
                    multiplier: prices.multiplier(role),
                })
            })
            .collect(),
        tokens_by_lane,
        tokens_other_lanes: other_lanes,
        tokens_by_milestone: by_milestone
            .iter()
            .map(|(milestone, tally)| MilestoneUsage {
                milestone: milestone.clone(),
                tokens: tally.tokens,
                cost: usd(tally),
            })
            .collect(),
        tokens_in_waste_by_cause: tokens_in_waste(events, &cards, &stop_spans),
        session_costs: session_costs(events, prices),
        reconciliation: reconcile(events),
        unpriced: unpriced
            .into_iter()
            .map(|(model, tokens)| UnpricedTokens { model, tokens })
            .collect(),
        budget: budget_meter(
            input.budget,
            input.launch_budget_usd,
            total.usd,
            prices.effective(),
            prices.currency(),
        ),
        model_comparison: model_comparison(events, &cards, prices, &stop_spans),
        overhead: overhead(&cards, &mission_level, input),
    }
}

fn session_costs(events: &[Event], prices: &PriceTable) -> Vec<SessionCost> {
    events
        .iter()
        .filter(|event| event.kind == EventKind::UsageReported)
        .filter_map(|event| {
            let usage = event.usage?;
            let model = event.model.as_ref().map(|m| m.0.as_str());
            Some(SessionCost {
                session: event.session_id.clone()?,
                role: event.role,
                model: event.model.clone(),
                cost: prices.session_cost(&usage, model)?,
            })
        })
        .collect()
}

fn overhead(
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
    level: &Tally,
    input: &CostInput<'_>,
) -> Overhead {
    let own = cards.get(&None);
    let active_s = own.map_or(0.0, |card| {
        card.intervals
            .iter()
            .filter(|interval| interval.state == TimeState::Active)
            .map(|interval| interval.length().as_secs_f64())
            .sum()
    });
    let waste_s = WasteCause::ALL
        .into_iter()
        .map(|cause| (cause, own.map_or(0, |card| card.waste.seconds(cause))))
        .collect();
    let apportioned = input.apportionment == OverheadApportionment::Equal;
    let features: Vec<&FeatureId> = cards.keys().filter_map(Option::as_ref).collect();
    let count = features.len().max(1) as f64;
    let share = |amount: f64| amount / count;
    Overhead {
        label: if apportioned {
            "apportioned"
        } else {
            "separate"
        }
        .to_owned(),
        tokens: level.tokens,
        cost: input.prices.cost(level.usd, CostUnit::Usd),
        active_s,
        waste_s,
        per_feature: if apportioned {
            features
                .into_iter()
                .map(|feature| OverheadShare {
                    feature: feature.clone(),
                    cost: input.prices.cost(share(level.usd), CostUnit::Usd),
                    active_s: share(active_s),
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}
