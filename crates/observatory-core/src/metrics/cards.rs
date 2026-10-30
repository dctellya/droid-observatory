//! The time of every card of a mission: the partition, rework and waste of each feature, and of
//! the events no feature owns (the mission level).

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use super::notes::{Notes, events_of_feature};
use super::rework::{
    ReworkSpan, ReworkWindow, first_pass_and_rework, merge_windows, rework_windows,
};
use super::span::Span;
use super::state::{Interval, PartitionContext, active_spans, partition_in};
use super::waste::{WasteTotals, waste_by_cause};
use crate::model::{CycleTemplate, Event, FeatureId, MilestoneId};

#[derive(Clone, Debug)]
pub struct CardTime {
    pub intervals: Vec<Interval>,
    pub windows: Vec<ReworkWindow>,
    pub rework: Vec<ReworkSpan>,
    /// The windows no feature owns whose milestone is the feature's (a red PR check names a
    /// milestone): the card's Active time inside them is rework, but they are not its windows.
    pub milestone_rework: Vec<ReworkSpan>,
    pub first_pass: Duration,
    pub rework_time: Duration,
    pub waste: WasteTotals,
}

impl CardTime {
    /// The spans whose Active time is rework: the card's own and its milestone's.
    pub fn rework_spans(&self) -> Vec<ReworkSpan> {
        self.rework
            .iter()
            .chain(&self.milestone_rework)
            .cloned()
            .collect()
    }
}

/// Every feature named by an event, and `None` for the events that name none (the mission's own
/// orchestration and its waits). `throttled` leaves the partition empty (F-033).
pub fn card_times(
    events: &[Event],
    template: &CycleTemplate,
    notes: &Notes,
    end: crate::model::Timestamp,
    throttled: &[Span],
) -> BTreeMap<Option<FeatureId>, CardTime> {
    let features: BTreeSet<FeatureId> =
        events.iter().filter_map(|e| e.feature_id.clone()).collect();
    let mut cards: Vec<(Option<FeatureId>, Vec<Event>)> = features
        .iter()
        .map(|feature| {
            (
                Some(feature.clone()),
                events_of_feature(events, feature, notes),
            )
        })
        .collect();
    let level = mission_level(events, notes);
    let level_spans = merge_windows(&rework_windows(&level, template, notes, end));
    let milestones_of = |feature: &FeatureId| -> BTreeSet<&MilestoneId> {
        events
            .iter()
            .filter(|event| event.feature_id.as_ref() == Some(feature))
            .filter_map(|event| event.milestone_id.as_ref())
            .collect()
    };
    cards.push((None, level));

    let actives: Vec<Vec<Span>> = cards.iter().map(|(_, own)| active_spans(own)).collect();
    let mut found = BTreeMap::new();
    for (index, (key, own)) in cards.iter().enumerate() {
        // Queued behind a feature, not behind the mission's own orchestration.
        let others: Vec<Span> = actives
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index && cards[*other].0.is_some())
            .flat_map(|(_, spans)| spans.iter().copied())
            .collect();
        let context = PartitionContext {
            others_active: &others,
            throttled,
        };
        let intervals = partition_in(own, end, &context);
        let windows = rework_windows(own, template, notes, end);
        let rework = merge_windows(&windows);
        let milestone_rework: Vec<ReworkSpan> = match key {
            Some(feature) => {
                let milestones = milestones_of(feature);
                level_spans
                    .iter()
                    .filter(|span| {
                        span.milestone
                            .as_ref()
                            .is_some_and(|m| milestones.contains(m))
                    })
                    .cloned()
                    .collect()
            }
            None => Vec::new(),
        };
        let all: Vec<ReworkSpan> = rework.iter().chain(&milestone_rework).cloned().collect();
        let (first_pass, rework_time) = first_pass_and_rework(&intervals, &all);
        let waste = waste_by_cause(&intervals, &all);
        found.insert(
            key.clone(),
            CardTime {
                intervals,
                windows,
                rework,
                milestone_rework,
                first_pass,
                rework_time,
                waste,
            },
        );
    }
    found
}

/// The events that name no feature, less the app answers to a feature's question, which
/// belong to that feature (`events_of_feature`).
fn mission_level(events: &[Event], notes: &Notes) -> Vec<Event> {
    let owned: BTreeSet<&str> = events
        .iter()
        .filter(|event| event.feature_id.is_some())
        .map(|event| event.id.0.as_str())
        .collect();
    let mut level: Vec<Event> = events
        .iter()
        .filter(|event| !event.kind.is_panel_only() && event.feature_id.is_none())
        .filter(|event| {
            !notes
                .question_of(event)
                .is_some_and(|question| owned.contains(question))
        })
        .cloned()
        .collect();
    level.sort_by_key(|event| event.at);
    level
}
