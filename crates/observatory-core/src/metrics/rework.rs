//! Rework windows (PRD §10.2): an open and a close event per kind, merged where they overlap on
//! one card, kept open to the mission's last event when nothing closes them.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::notes::Notes;
use super::span::{Span, gap, timeline, union};
use super::state::{Interval, TimeState};
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, LaneId, MilestoneId, Timestamp,
};

/// The four open and close pairs of PRD §10.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReworkKind {
    /// `finding.blocking` until `rework.resolved`.
    Finding,
    /// `qa.scenario.failed` until `qa.scenario.passed` for the same scenario.
    Scenario,
    /// `integration.conflict` until `integration.conflict_resolved`.
    Conflict,
    /// `check.failed` until `check.passed` for the same check.
    Check,
}

impl ReworkKind {
    pub const ALL: [ReworkKind; 4] = [Self::Finding, Self::Scenario, Self::Conflict, Self::Check];

    fn opened_by(kind: &EventKind) -> Option<Self> {
        match kind {
            EventKind::FindingBlocking => Some(Self::Finding),
            EventKind::QaScenarioFailed => Some(Self::Scenario),
            EventKind::IntegrationConflict => Some(Self::Conflict),
            EventKind::CheckFailed => Some(Self::Check),
            _ => None,
        }
    }

    fn closed_by(kind: &EventKind) -> Option<Self> {
        match kind {
            EventKind::ReworkResolved => Some(Self::Finding),
            EventKind::QaScenarioPassed => Some(Self::Scenario),
            EventKind::IntegrationConflictResolved => Some(Self::Conflict),
            EventKind::CheckPassed => Some(Self::Check),
            _ => None,
        }
    }

    fn opening_event(self) -> EventKind {
        match self {
            Self::Finding => EventKind::FindingBlocking,
            Self::Scenario => EventKind::QaScenarioFailed,
            Self::Conflict => EventKind::IntegrationConflict,
            Self::Check => EventKind::CheckFailed,
        }
    }
}

/// The lanes that can open a rework window, one per kind, as the template places the opening
/// events (`scrutiny`, `user_testing`, `integrate`, `gate` in the default template).
pub fn rework_causes(template: &CycleTemplate) -> Vec<LaneId> {
    ReworkKind::ALL
        .into_iter()
        .map(|kind| {
            template
                .fixed_lane(&kind.opening_event())
                .unwrap_or_else(|| template.first())
                .clone()
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReworkWindow {
    pub feature: Option<FeatureId>,
    /// The card of a window with no feature: a red PR check names the milestone it returns to.
    pub milestone: Option<MilestoneId>,
    pub kind: ReworkKind,
    /// The lane id of the lane that holds the opening event.
    pub cause: LaneId,
    pub opened_by: EventId,
    /// `None` is unresolved: the window runs to the end the caller passed.
    pub closed_by: Option<EventId>,
    pub from: Timestamp,
    pub to: Timestamp,
    /// The scenario or check the window is about, when the notes name it.
    pub key: Option<String>,
}

/// Windows of every card in `events`, oldest first. An event pairs with the oldest open window of
/// its kind; when both name a scenario or check, only the same one closes it; a red of a
/// check that already has an open window opens no second one.
pub fn rework_windows(
    events: &[Event],
    template: &CycleTemplate,
    notes: &Notes,
    end: Timestamp,
) -> Vec<ReworkWindow> {
    let mut cards: BTreeMap<(Option<FeatureId>, Option<MilestoneId>), Vec<&Event>> =
        BTreeMap::new();
    for event in timeline(events) {
        let milestone = event
            .feature_id
            .is_none()
            .then(|| event.milestone_id.clone())
            .flatten();
        cards
            .entry((event.feature_id.clone(), milestone))
            .or_default()
            .push(event);
    }

    let mut windows = Vec::new();
    for ((feature, milestone), card) in cards {
        let mut open: Vec<ReworkWindow> = Vec::new();
        for event in card {
            let key = notes.key(event);
            if let Some(kind) = ReworkKind::opened_by(&event.kind) {
                let duplicate = key.is_some()
                    && open.iter().any(|w| {
                        w.closed_by.is_none() && w.kind == kind && w.key.as_deref() == key
                    });
                if !duplicate {
                    open.push(ReworkWindow {
                        feature: feature.clone(),
                        milestone: milestone.clone(),
                        kind,
                        cause: template
                            .fixed_lane(&event.kind)
                            .unwrap_or(&event.lane)
                            .clone(),
                        opened_by: event.id.clone(),
                        closed_by: None,
                        from: event.at,
                        to: end.max(event.at),
                        key: key.map(str::to_owned),
                    });
                }
            } else if let Some(kind) = ReworkKind::closed_by(&event.kind)
                && let Some(window) = open.iter_mut().find(|w| {
                    w.closed_by.is_none()
                        && w.kind == kind
                        && key.zip(w.key.as_deref()).is_none_or(|(a, b)| a == b)
                })
            {
                window.closed_by = Some(event.id.clone());
                window.to = event.at;
            }
        }
        windows.extend(open);
    }
    windows.sort_by_key(|window| window.from);
    windows
}

/// The number of fix loops: the windows a red check opened.
pub fn fix_loops(windows: &[ReworkWindow]) -> usize {
    windows
        .iter()
        .filter(|window| window.kind == ReworkKind::Check)
        .count()
}

/// Overlapping windows of one card reported as one span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReworkSpan {
    pub feature: Option<FeatureId>,
    pub milestone: Option<MilestoneId>,
    pub from: Timestamp,
    pub to: Timestamp,
    /// The windows merged into the span.
    pub windows: usize,
    /// Some window in it has no closing event.
    pub unresolved: bool,
    pub causes: Vec<LaneId>,
}

pub fn merge_windows(windows: &[ReworkWindow]) -> Vec<ReworkSpan> {
    let mut sorted: Vec<&ReworkWindow> = windows.iter().collect();
    sorted.sort_by_key(|window| (&window.feature, &window.milestone, window.from, window.to));
    let mut spans: Vec<ReworkSpan> = Vec::new();
    for window in sorted {
        let unresolved = window.closed_by.is_none();
        match spans.last_mut() {
            Some(last)
                if last.feature == window.feature
                    && last.milestone == window.milestone
                    && window.from < last.to =>
            {
                last.to = last.to.max(window.to);
                last.windows += 1;
                last.unresolved |= unresolved;
                if !last.causes.contains(&window.cause) {
                    last.causes.push(window.cause.clone());
                }
            }
            _ => spans.push(ReworkSpan {
                feature: window.feature.clone(),
                milestone: window.milestone.clone(),
                from: window.from,
                to: window.to,
                windows: 1,
                unresolved,
                causes: vec![window.cause.clone()],
            }),
        }
    }
    spans
}

/// Active time outside and inside the spans: `(first_pass, rework)`; they add up to the Active
/// time of the intervals (PRD §10.2).
pub fn first_pass_and_rework(intervals: &[Interval], spans: &[ReworkSpan]) -> (Duration, Duration) {
    let covered = union(
        spans
            .iter()
            .map(|span| Span {
                from: span.from,
                to: span.to,
            })
            .collect(),
    );
    let (mut first_pass, mut rework) = (Duration::ZERO, Duration::ZERO);
    for interval in intervals.iter().filter(|i| i.state == TimeState::Active) {
        let inside: Duration = covered
            .iter()
            .map(|span| gap(span.from.max(interval.from), span.to.min(interval.to)))
            .sum();
        rework += inside;
        first_pass += interval.length().saturating_sub(inside);
    }
    (first_pass, rework)
}
