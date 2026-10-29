//! Model comparison (FR-M7): per role and model, cost and first-pass active time per completed
//! feature.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::cards::CardTime;
use super::price::{Cost, CostUnit, PriceTable};
use super::span::{Span, event_span, overlap, timeline, union};
use super::tokens::{TokenCounts, turns};
use crate::model::{Event, EventKind, FeatureId, ModelId, Role};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelComparisonRow {
    pub role: Role,
    pub model: Option<ModelId>,
    /// Features with `task.done` and `scrutiny.passed` and no unresolved rework window: the
    /// divisor of both figures, the same on every row.
    pub completed_features: u64,
    /// The role-model's whole cost of the run, from its turns.
    pub cost: Cost,
    pub tokens: TokenCounts,
    pub cost_per_completed_feature: Option<f64>,
    /// The role-model's event spans outside rework windows and stops, in seconds.
    pub first_pass_active_s: f64,
    pub first_pass_active_s_per_completed_feature: Option<f64>,
}

/// The features whose work is done and validated. PRD §10.4 also asks for the integration; the
/// mapping emits no `integration.merged` for a feature, so it is not required here.
pub fn completed_features(
    events: &[Event],
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
) -> BTreeSet<FeatureId> {
    let has = |feature: &FeatureId, kind: EventKind| {
        events
            .iter()
            .any(|e| e.feature_id.as_ref() == Some(feature) && e.kind == kind)
    };
    cards
        .iter()
        .filter_map(|(key, card)| key.as_ref().map(|feature| (feature, card)))
        .filter(|(feature, card)| {
            has(feature, EventKind::TaskDone)
                && has(feature, EventKind::ScrutinyPassed)
                && !card.rework.iter().any(|span| span.unresolved)
        })
        .map(|(feature, _)| feature.clone())
        .collect()
}

#[derive(Default)]
struct Group {
    tokens: TokenCounts,
    usd: f64,
    first_pass: f64,
}

pub fn model_comparison(
    events: &[Event],
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
    prices: &PriceTable,
    throttled: &[Span],
) -> Vec<ModelComparisonRow> {
    let mut groups: BTreeMap<(usize, Option<String>), Group> = BTreeMap::new();
    let rank = |role: Role| {
        Role::ALL
            .iter()
            .position(|r| *r == role)
            .unwrap_or(usize::MAX)
    };

    for turn in turns(events) {
        let key = (rank(turn.event.role), turn.model.map(|m| m.0.clone()));
        let group = groups.entry(key).or_default();
        group.tokens += turn.tokens;
        group.usd += prices
            .turn_usd(
                turn.event.role,
                turn.model.map(|m| m.0.as_str()),
                &turn.tokens,
            )
            .unwrap_or(0.0);
    }

    let stops = union(throttled.to_vec());
    for (feature, card) in cards {
        let rework: Vec<Span> = union(
            card.rework
                .iter()
                .map(|span| Span {
                    from: span.from,
                    to: span.to,
                })
                .collect(),
        );
        let mut spans: BTreeMap<(usize, String), Vec<Span>> = BTreeMap::new();
        for event in timeline(events)
            .into_iter()
            .filter(|e| &e.feature_id == feature)
            .filter(|e| !matches!(e.kind, EventKind::ToolError | EventKind::HookBlocked))
        {
            if let (Some(span), Some(model)) = (event_span(event), &event.model) {
                spans
                    .entry((rank(event.role), model.0.clone()))
                    .or_default()
                    .push(span);
            }
        }
        for ((role, model), own) in spans {
            let own = union(own);
            let seconds = own
                .iter()
                .map(|span| span.length())
                .sum::<std::time::Duration>()
                .saturating_sub(overlap(&own, &rework))
                .saturating_sub(overlap(&own, &stops));
            groups.entry((role, Some(model))).or_default().first_pass += seconds.as_secs_f64();
        }
    }

    let completed = completed_features(events, cards).len() as u64;
    let per = |total: f64| (completed > 0).then(|| total / completed as f64);
    groups
        .into_iter()
        .map(|((role, model), group)| ModelComparisonRow {
            role: Role::ALL.get(role).copied().unwrap_or(Role::Unresolved),
            model: model.map(ModelId),
            completed_features: completed,
            cost: prices.cost(group.usd, CostUnit::Usd),
            tokens: group.tokens,
            cost_per_completed_feature: per(group.usd),
            first_pass_active_s: group.first_pass,
            first_pass_active_s_per_completed_feature: per(group.first_pass),
        })
        .collect()
}
