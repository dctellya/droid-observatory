//! `MetricsReport` (architecture §9): the rows of the CSV, the mission summary, the overhead line,
//! `throttled` and the cost half, made at one instant over the events up to the hand-over.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use serde::Serialize;

use super::bottleneck::{Bottleneck, Waiting};
use super::cards::card_times;
use super::cycle::{ClockKinds, Cycle, StageTime, add_stages, cycles};
use super::intervals::{CardIntervals, card_intervals};
use super::report::{CostInput, CostReport, Overhead, cost_report};
use super::rows::{Context, MetricsRow, RowScope, round4, rows, whole_seconds};
use super::span::union;
use super::throttled::{Throttled, throttled};
use super::waste::WasteCause;
use crate::config::OverheadApportionment;
use crate::model::{Event, FeatureId, MilestoneId, Timestamp};

/// The run as a whole: its clock, its stage times, what it spent and where it waited.
#[derive(Clone, Debug, Serialize)]
pub struct MissionSummary {
    pub cycle: Cycle,
    pub features: usize,
    /// Features that had no `feature.defined` and started at their first `task.started`.
    pub fallback_features: usize,
    pub milestones: usize,
    pub first_pass_active_s: u64,
    pub rework_s: u64,
    pub waste_s: BTreeMap<WasteCause, u64>,
    pub fix_loops: usize,
    pub throttled_minutes: u64,
    pub stage_s: Vec<StageTime>,
    pub bottleneck: Option<Bottleneck>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MetricsReport {
    /// One row per feature, then one per milestone.
    pub rows: Vec<MetricsRow>,
    pub mission: MissionSummary,
    /// `throttled` apart from the eight causes and the five groups: the run's minutes, each
    /// milestone's and the intervals with their reasons.
    pub throttled: Throttled,
    /// Events after the last `handed_to_cd`: counted, never read (D-024).
    pub ignored_after_handed_to_cd: usize,
    /// The partition and rework windows of every card, the mission-level card included. JSON
    /// only: the CSV keeps its columns.
    pub intervals: Vec<CardIntervals>,
    #[serde(flatten)]
    pub cost: CostReport,
}

impl Deref for MetricsReport {
    type Target = CostReport;

    fn deref(&self) -> &CostReport {
        &self.cost
    }
}

/// The events up to and including the last event that stops the cycle clock, how many non-turn
/// events follow it, and the instant of that last hand-over.
fn until_hand_over(input: &CostInput<'_>) -> (Vec<Event>, usize, Option<Timestamp>) {
    let clock = ClockKinds::of(input.template);
    let Some(last) = input
        .events
        .iter()
        .filter(|event| clock.ends(event))
        .map(|event| event.at)
        .max()
    else {
        return (input.events.to_vec(), 0, None);
    };
    let (kept, dropped): (Vec<&Event>, Vec<&Event>) =
        input.events.iter().partition(|event| event.at <= last);
    let ignored = dropped
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .count();
    (kept.into_iter().cloned().collect(), ignored, Some(last))
}

pub fn metrics_report(input: &CostInput<'_>) -> MetricsReport {
    let (events, ignored, handed_over) = until_hand_over(input);
    let end = handed_over.map_or(input.end, |at| at.min(input.end));
    let cut = CostInput {
        events: &events,
        template: input.template,
        notes: input.notes,
        prices: input.prices,
        end,
        budget: input.budget,
        launch_budget_usd: input.launch_budget_usd,
        apportionment: input.apportionment,
    };
    let cost = cost_report(&cut);
    let stops = throttled(&events, input.notes, end);
    let spans = stops.spans();
    let cards = card_times(&events, input.template, input.notes, end, &spans);
    let table = cycles(&events, input.template, end);
    let disjoint = union(spans);
    let context = Context {
        events: &events,
        template: input.template,
        prices: input.prices,
        notes: input.notes,
        cards: &cards,
        cycles: &table,
        stops: &disjoint,
        end,
    };
    let (mut found, members) = rows(&context);
    if input.apportionment == OverheadApportionment::Equal {
        apportion(&mut found, &cost.overhead, &members);
    }
    let mission = summary(
        &found,
        &table.mission,
        table.fallback_features,
        input,
        &stops,
    );
    MetricsReport {
        rows: found,
        mission,
        throttled: stops,
        ignored_after_handed_to_cd: ignored,
        intervals: card_intervals(&cards, &members),
        cost,
    }
}

fn summary(
    rows: &[MetricsRow],
    cycle: &Cycle,
    fallback_features: usize,
    input: &CostInput<'_>,
    stops: &Throttled,
) -> MissionSummary {
    let features = || rows.iter().filter(|row| row.scope == RowScope::Feature);
    let mut stage_s: Vec<StageTime> = input
        .template
        .lanes
        .iter()
        .map(|lane| StageTime {
            lane: lane.id.clone(),
            seconds: 0,
        })
        .collect();
    let mut waiting = Waiting::new(input.template);
    let mut waste_s: BTreeMap<WasteCause, u64> = WasteCause::ALL
        .into_iter()
        .map(|cause| (cause, 0))
        .collect();
    for row in features() {
        add_stages(&mut stage_s, &row.stage_s);
        waiting.add(&row.waiting);
        for (cause, seconds) in &row.waste_s {
            *waste_s.entry(*cause).or_default() += seconds;
        }
    }
    MissionSummary {
        cycle: *cycle,
        features: features().count(),
        fallback_features,
        milestones: rows
            .iter()
            .filter(|row| row.scope == RowScope::Milestone)
            .count(),
        first_pass_active_s: features().map(|row| row.first_pass_active_s).sum(),
        rework_s: features().map(|row| row.rework_s).sum(),
        waste_s,
        fix_loops: rows
            .iter()
            .filter(|row| row.scope == RowScope::Milestone)
            .map(|row| row.fix_loops)
            .sum(),
        throttled_minutes: stops.mission_minutes,
        stage_s,
        bottleneck: waiting.bottleneck(input.template),
    }
}

/// `total` split over `count` shares so that the shares add up to `total`.
fn share(total: u64, count: usize, index: usize) -> u64 {
    let count = count.max(1) as u64;
    total / count + u64::from((index as u64) < total % count)
}

/// `[metrics].overhead_apportionment = "equal"`: the mission's own seconds, tokens and cost go to
/// the features in equal parts, and to each milestone as the sum of its features' parts.
fn apportion(
    rows: &mut [MetricsRow],
    overhead: &Overhead,
    members: &BTreeMap<MilestoneId, BTreeSet<FeatureId>>,
) {
    let features: Vec<String> = rows
        .iter()
        .filter(|row| row.scope == RowScope::Feature)
        .map(|row| row.id.clone())
        .collect();
    let count = features.len();
    let active = whole_seconds(std::time::Duration::from_secs_f64(
        overhead.active_s.max(0.0),
    ));
    let usd = overhead.cost.amount / count.max(1) as f64;
    for row in rows.iter_mut() {
        let owned: Vec<usize> = match row.scope {
            RowScope::Feature => features
                .iter()
                .position(|id| *id == row.id)
                .into_iter()
                .collect(),
            RowScope::Milestone => members
                .get(&MilestoneId(row.id.clone()))
                .map(|set| {
                    set.iter()
                        .filter_map(|id| features.iter().position(|f| *f == id.0))
                        .collect()
                })
                .unwrap_or_default(),
        };
        for index in owned {
            row.first_pass_active_s += share(active, count, index);
            for (cause, seconds) in &overhead.waste_s {
                *row.waste_s.entry(*cause).or_default() += share(*seconds, count, index);
            }
            row.tokens_input += share(overhead.tokens.input, count, index);
            row.tokens_output += share(overhead.tokens.output, count, index);
            row.tokens_cache_read += share(overhead.tokens.cache_read, count, index);
            row.cost_usd = round4(row.cost_usd + usd);
        }
    }
}
