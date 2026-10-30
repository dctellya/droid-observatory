//! One row per feature and per milestone: the columns of the CSV contract and the stage times and
//! bottleneck that only the JSON carries.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::Serialize;

use super::bottleneck::{Bottleneck, Waiting};
use super::cards::CardTime;
use super::cycle::{Cycle, Cycles, StageTime, add_stages, stage_times};
use super::notes::{Notes, events_of_feature};
use super::price::PriceTable;
use super::rework::{ReworkKind, fix_loops};
use super::span::{Span, overlap};
use super::throttled::minutes;
use super::tokens::{TokenCounts, turns};
use super::waste::WasteCause;
use crate::model::{CycleTemplate, Event, EventKind, FeatureId, MilestoneId, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RowScope {
    Feature,
    Milestone,
}

impl RowScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Milestone => "milestone",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MetricsRow {
    pub scope: RowScope,
    pub id: String,
    pub name: String,
    pub cycle_s: u64,
    pub cycle: Cycle,
    pub first_pass_active_s: u64,
    pub rework_s: u64,
    /// The eight measured causes, every one present; `throttled` is not one of them.
    pub waste_s: BTreeMap<WasteCause, u64>,
    pub rework_windows: usize,
    pub fix_loops: usize,
    pub throttled_minutes: u64,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub tokens_cache_read: u64,
    /// From the sidecar's `usage.reported`; the usage log carries no cache-write count.
    pub tokens_cache_write: u64,
    /// Rounded to four decimals so the CSV and the JSON print the same figure.
    pub cost_usd: f64,
    pub price_table: String,
    pub stage_s: Vec<StageTime>,
    pub bottleneck: Option<Bottleneck>,
    #[serde(skip)]
    pub(super) waiting: Waiting,
}

pub(super) fn round4(amount: f64) -> f64 {
    (amount * 1e4).round() / 1e4
}

pub(super) fn whole_seconds(time: Duration) -> u64 {
    u64::try_from((time.as_millis() + 500) / 1000).unwrap_or(u64::MAX)
}

/// What the rows read besides the events.
pub(super) struct Context<'a> {
    pub events: &'a [Event],
    pub template: &'a CycleTemplate,
    pub prices: &'a PriceTable,
    pub notes: &'a Notes,
    pub cards: &'a BTreeMap<Option<FeatureId>, CardTime>,
    pub cycles: &'a Cycles,
    /// Disjoint spans in which the platform stopped the work.
    pub stops: &'a [Span],
    pub end: Timestamp,
}

#[derive(Clone, Copy, Default)]
struct Usage {
    tokens: TokenCounts,
    cache_write: u64,
    usd: f64,
}

impl Usage {
    fn add_turn(&mut self, tokens: TokenCounts, usd: Option<f64>) {
        self.tokens += tokens;
        self.usd += usd.unwrap_or(0.0);
    }
}

struct UsageTables {
    by_feature: BTreeMap<FeatureId, Usage>,
    by_milestone: BTreeMap<MilestoneId, Usage>,
}

/// Turns give input, output, cache read and cost; the sidecar's usage reports give cache write
/// (the same sources the Details Cost tab reads, so the two agree).
fn usage_tables(events: &[Event], prices: &PriceTable) -> UsageTables {
    let mut found = UsageTables {
        by_feature: BTreeMap::new(),
        by_milestone: BTreeMap::new(),
    };
    for turn in turns(events) {
        let usd = prices.turn_usd(
            turn.event.role,
            turn.model.map(|model| model.0.as_str()),
            &turn.tokens,
        );
        if let Some(feature) = &turn.event.feature_id {
            found
                .by_feature
                .entry(feature.clone())
                .or_default()
                .add_turn(turn.tokens, usd);
        }
        if let Some(milestone) = &turn.event.milestone_id {
            found
                .by_milestone
                .entry(milestone.clone())
                .or_default()
                .add_turn(turn.tokens, usd);
        }
    }
    for event in events.iter().filter(|e| e.kind == EventKind::UsageReported) {
        let Some(usage) = event.usage else { continue };
        if let Some(feature) = &event.feature_id {
            found
                .by_feature
                .entry(feature.clone())
                .or_default()
                .cache_write += usage.cache_write;
        }
        if let Some(milestone) = &event.milestone_id {
            found
                .by_milestone
                .entry(milestone.clone())
                .or_default()
                .cache_write += usage.cache_write;
        }
    }
    found
}

fn throttled_in(cycle: &Cycle, context: &Context<'_>) -> u64 {
    cycle.window(context.end).map_or(0, |(from, to)| {
        minutes(overlap(&[Span { from, to }], context.stops))
    })
}

fn new_row(
    scope: RowScope,
    id: &str,
    cycle: Cycle,
    usage: Usage,
    context: &Context<'_>,
) -> MetricsRow {
    MetricsRow {
        scope,
        id: id.to_owned(),
        name: id.to_owned(),
        cycle_s: cycle.seconds,
        cycle,
        first_pass_active_s: 0,
        rework_s: 0,
        waste_s: WasteCause::ALL
            .into_iter()
            .map(|cause| (cause, 0))
            .collect(),
        rework_windows: 0,
        fix_loops: 0,
        throttled_minutes: throttled_in(&cycle, context),
        tokens_input: usage.tokens.input,
        tokens_output: usage.tokens.output,
        tokens_cache_read: usage.tokens.cache_read,
        tokens_cache_write: usage.cache_write,
        cost_usd: round4(usage.usd),
        price_table: context.prices.effective().to_owned(),
        stage_s: context
            .template
            .lanes
            .iter()
            .map(|lane| StageTime {
                lane: lane.id.clone(),
                seconds: 0,
            })
            .collect(),
        bottleneck: None,
        waiting: Waiting::new(context.template),
    }
}

fn feature_row(
    feature: &FeatureId,
    cycle: Cycle,
    usage: Usage,
    context: &Context<'_>,
) -> MetricsRow {
    let mut row = new_row(RowScope::Feature, &feature.0, cycle, usage, context);
    let own = events_of_feature(context.events, feature, context.notes);
    row.stage_s = stage_times(&own, context.template);
    if let Some(card) = context.cards.get(&Some(feature.clone())) {
        row.first_pass_active_s = whole_seconds(card.first_pass);
        row.rework_s = whole_seconds(card.rework_time);
        for cause in WasteCause::ALL {
            row.waste_s.insert(cause, card.waste.seconds(cause));
        }
        row.rework_windows = card.rework.len();
        row.fix_loops = fix_loops(&card.windows);
        row.waiting = Waiting::of_card(context.template, &own, &card.intervals);
    }
    row.bottleneck = row.waiting.bottleneck(context.template);
    row
}

/// The rows of every feature, oldest id first, and of every milestone: a milestone is the sum of
/// the features that have an event in it, and of the gate windows no feature owns.
pub(super) fn rows(
    context: &Context<'_>,
) -> (Vec<MetricsRow>, BTreeMap<MilestoneId, BTreeSet<FeatureId>>) {
    let usage = usage_tables(context.events, context.prices);
    let mut members: BTreeMap<MilestoneId, BTreeSet<FeatureId>> = BTreeMap::new();
    for event in context.events {
        if let (Some(milestone), Some(feature)) = (&event.milestone_id, &event.feature_id) {
            members
                .entry(milestone.clone())
                .or_default()
                .insert(feature.clone());
        }
    }
    for milestone in context.cycles.milestones.keys() {
        members.entry(milestone.clone()).or_default();
    }

    let mut out: Vec<MetricsRow> = context
        .cycles
        .features
        .iter()
        .map(|(feature, cycle)| {
            let used = usage.by_feature.get(feature).copied().unwrap_or_default();
            feature_row(feature, *cycle, used, context)
        })
        .collect();
    let level = context.cards.get(&None);
    let mut milestones = Vec::new();
    for (milestone, features) in &members {
        let cycle = context
            .cycles
            .milestones
            .get(milestone)
            .copied()
            .unwrap_or(Cycle::NEVER);
        let used = usage
            .by_milestone
            .get(milestone)
            .copied()
            .unwrap_or_default();
        let mut row = new_row(RowScope::Milestone, &milestone.0, cycle, used, context);
        for own in out
            .iter()
            .filter(|r| features.contains(&FeatureId(r.id.clone())))
        {
            row.first_pass_active_s += own.first_pass_active_s;
            row.rework_s += own.rework_s;
            for (cause, seconds) in &own.waste_s {
                *row.waste_s.entry(*cause).or_default() += seconds;
            }
            row.rework_windows += own.rework_windows;
            row.fix_loops += own.fix_loops;
            add_stages(&mut row.stage_s, &own.stage_s);
            row.waiting.add(&own.waiting);
        }
        if let Some(level) = level {
            row.rework_windows += level
                .rework
                .iter()
                .filter(|span| span.milestone.as_ref() == Some(milestone))
                .count();
            row.fix_loops += level
                .windows
                .iter()
                .filter(|w| w.kind == ReworkKind::Check && w.milestone.as_ref() == Some(milestone))
                .count();
        }
        row.bottleneck = row.waiting.bottleneck(context.template);
        milestones.push(row);
    }
    out.extend(milestones);
    (out, members)
}
