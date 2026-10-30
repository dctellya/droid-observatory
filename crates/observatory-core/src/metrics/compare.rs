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
    /// Features with `task.done` and `scrutiny.passed` and no unresolved rework window, and
    /// integrated where `integrated_observable`: the divisor of both figures, the same on every row.
    pub completed_features: u64,
    /// The capture holds an `integration.merged` event, so the count applies the `integrated`
    /// condition; `false` means the condition was waived (a session capture records no merge).
    pub integrated_observable: bool,
    /// The role-model's whole cost of the run, from its turns.
    pub cost: Cost,
    pub tokens: TokenCounts,
    pub cost_per_completed_feature: Option<f64>,
    /// The role-model's event spans outside rework windows and stops, in seconds.
    pub first_pass_active_s: f64,
    pub first_pass_active_s_per_completed_feature: Option<f64>,
}

/// Whether the capture records any integration: only then can the `integrated` condition of the
/// completed-feature count be applied. The M0 session capture holds no `integration.merged`.
pub fn integration_observable(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| event.kind == EventKind::IntegrationMerged)
}

/// The features whose work is done and validated (`task.done`, `scrutiny.passed`, no unresolved
/// rework window) and, where the capture records integrations, integrated (PRD §10.4): an
/// `integration.merged` at or after the feature's last `task.done` that names the feature or, when
/// it names no feature, the feature's milestone. Without any such event the condition is waived.
pub fn completed_features(
    events: &[Event],
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
) -> BTreeSet<FeatureId> {
    let has = |feature: &FeatureId, kind: EventKind| {
        events
            .iter()
            .any(|e| e.feature_id.as_ref() == Some(feature) && e.kind == kind)
    };
    let integrated_observable = integration_observable(events);
    cards
        .iter()
        .filter_map(|(key, card)| key.as_ref().map(|feature| (feature, card)))
        .filter(|(feature, card)| {
            has(feature, EventKind::TaskDone)
                && has(feature, EventKind::ScrutinyPassed)
                && !card.rework.iter().any(|span| span.unresolved)
                && (!integrated_observable || integrated_after_done(events, feature))
        })
        .map(|(feature, _)| feature.clone())
        .collect()
}

fn integrated_after_done(events: &[Event], feature: &FeatureId) -> bool {
    let own = || {
        events
            .iter()
            .filter(|e| e.feature_id.as_ref() == Some(feature))
    };
    let Some(done) = own()
        .filter(|e| e.kind == EventKind::TaskDone)
        .map(|e| e.at)
        .max()
    else {
        return false;
    };
    let milestone = own().find_map(|e| e.milestone_id.as_ref());
    events
        .iter()
        .filter(|e| e.kind == EventKind::IntegrationMerged && e.at >= done)
        .any(|merge| match &merge.feature_id {
            Some(named) => named == feature,
            None => milestone.is_some() && merge.milestone_id.as_ref() == milestone,
        })
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

    // The usage log has no cache-write count: that class is priced from each session report.
    for event in events.iter().filter(|e| e.kind == EventKind::UsageReported) {
        let Some(usage) = event.usage.filter(|usage| usage.cache_write > 0) else {
            continue;
        };
        let key = (rank(event.role), event.model.as_ref().map(|m| m.0.clone()));
        groups.entry(key).or_default().usd += prices
            .cache_write_usd(
                event.role,
                event.model.as_ref().map(|m| m.0.as_str()),
                usage.cache_write,
            )
            .unwrap_or(0.0);
    }

    let stops = union(throttled.to_vec());
    for (feature, card) in cards {
        let rework: Vec<Span> = union(
            card.rework_spans()
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
    let integrated_observable = integration_observable(events);
    let per = |total: f64| (completed > 0).then(|| total / completed as f64);
    groups
        .into_iter()
        .map(|((role, model), group)| ModelComparisonRow {
            role: Role::ALL.get(role).copied().unwrap_or(Role::Unresolved),
            model: model.map(ModelId),
            completed_features: completed,
            integrated_observable,
            cost: prices.cost(group.usd, CostUnit::Usd),
            tokens: group.tokens,
            cost_per_completed_feature: per(group.usd),
            first_pass_active_s: group.first_pass,
            first_pass_active_s_per_completed_feature: per(group.first_pass),
        })
        .collect()
}
