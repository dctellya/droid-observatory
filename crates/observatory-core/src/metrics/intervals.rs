//! The partition of every card as the report carries it: what `card_times` measured, unchanged,
//! so a reader can see where each state starts and ends (an interval that closes at an answer
//! ends at that answer's `at`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::cards::CardTime;
use super::rework::{ReworkKind, ReworkWindow};
use super::state::{Interval, TimeState};
use super::waste::WasteCause;
use crate::model::{EventId, FeatureId, LaneId, MilestoneId, Timestamp};

/// One interval of a card's partition and the waste cause of its state (`None` for Active, whose
/// share inside a rework window is in the card's `rework` list).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IntervalRow {
    pub from: Timestamp,
    pub to: Timestamp,
    pub state: TimeState,
    pub cause: Option<WasteCause>,
}

impl From<&Interval> for IntervalRow {
    fn from(interval: &Interval) -> Self {
        Self {
            from: interval.from,
            to: interval.to,
            state: interval.state,
            cause: interval.state.waste_cause(),
        }
    }
}

/// One rework window of a card, from its opening event to its closing one (or the report's end).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReworkWindowRow {
    pub kind: ReworkKind,
    /// The lane id of the lane that holds the opening event.
    pub cause: LaneId,
    pub opened_by: EventId,
    pub closed_by: Option<EventId>,
    pub from: Timestamp,
    pub to: Timestamp,
    pub key: Option<String>,
}

impl From<&ReworkWindow> for ReworkWindowRow {
    fn from(window: &ReworkWindow) -> Self {
        Self {
            kind: window.kind,
            cause: window.cause.clone(),
            opened_by: window.opened_by.clone(),
            closed_by: window.closed_by.clone(),
            from: window.from,
            to: window.to,
            key: window.key.clone(),
        }
    }
}

/// A card's partition and rework windows. `feature: None` is the mission-level card, the events
/// that name no feature (a hosted orchestrator's question, for one).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CardIntervals {
    pub feature: Option<FeatureId>,
    pub milestone: Option<MilestoneId>,
    pub intervals: Vec<IntervalRow>,
    pub rework: Vec<ReworkWindowRow>,
}

/// The cards in the order of `card_times`: the mission-level card, then the features by id.
pub fn card_intervals(
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
    members: &BTreeMap<MilestoneId, BTreeSet<FeatureId>>,
) -> Vec<CardIntervals> {
    cards
        .iter()
        .map(|(feature, card)| CardIntervals {
            feature: feature.clone(),
            milestone: feature.as_ref().and_then(|feature| {
                members
                    .iter()
                    .find(|(_, features)| features.contains(feature))
                    .map(|(milestone, _)| milestone.clone())
            }),
            intervals: card.intervals.iter().map(IntervalRow::from).collect(),
            rework: card.windows.iter().map(ReworkWindowRow::from).collect(),
        })
        .collect()
}
