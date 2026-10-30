//! Engineering cycle time (F-028) and stage times. The clock is the template's: it names the kind
//! that starts it, the kind that starts it for a feature the plan never defined, and the kind
//! that stops it (`handed_to_cd`); the lanes are the template's.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::span::{event_span, gap, timeline};
use crate::model::{CycleTemplate, Event, EventKind, FeatureId, LaneId, MilestoneId, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CycleEnd {
    HandedToCd,
    /// A milestone with no PR yet ends at its integration into the mission branch.
    Integration,
    /// No end yet: the clock reads the instant the report is made at.
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Cycle {
    pub start: Option<Timestamp>,
    pub end: Option<Timestamp>,
    /// Whole seconds from `start` to `end`, or to the report's end while open.
    pub seconds: u64,
    pub ends_at: CycleEnd,
    /// The feature had no `feature.defined`: its first `task.started` is the start.
    pub fallback: bool,
}

impl Cycle {
    /// A clock that never started.
    pub const NEVER: Cycle = Cycle {
        start: None,
        end: None,
        seconds: 0,
        ends_at: CycleEnd::Open,
        fallback: false,
    };

    /// The span the clock covers, up to `now` while it is open.
    pub fn window(&self, now: Timestamp) -> Option<(Timestamp, Timestamp)> {
        let start = self.start?;
        Some((start, self.end.unwrap_or(now).max(start)))
    }
}

fn whole_seconds(from: Timestamp, to: Timestamp) -> u64 {
    u64::try_from((gap(from, to).as_millis() + 500) / 1000).unwrap_or(u64::MAX)
}

fn finish(
    start: Option<(Timestamp, bool)>,
    end: Option<(Timestamp, CycleEnd)>,
    now: Timestamp,
) -> Cycle {
    let Some((start, fallback)) = start else {
        return Cycle::NEVER;
    };
    let (end, ends_at) = match end {
        Some((at, how)) if at >= start => (Some(at), how),
        _ => (None, CycleEnd::Open),
    };
    Cycle {
        start: Some(start),
        end,
        seconds: whole_seconds(start, end.unwrap_or(now)),
        ends_at,
        fallback,
    }
}

/// The cycle clock of a template: the kinds that start, fall back and stop it. A template with
/// no clock that has both a start and an end falls back to the cycle's first event kind pair of
/// the PRD (`feature.defined`, `task.started`) and the template's final event.
pub struct ClockKinds {
    start: String,
    fallback: Option<String>,
    end: String,
}

impl ClockKinds {
    pub fn of(template: &CycleTemplate) -> Self {
        match template
            .clocks
            .iter()
            .find(|clock| clock.start.is_some() && clock.end.is_some())
        {
            Some(clock) => Self {
                start: clock.start.clone().unwrap_or_default(),
                fallback: clock.start_fallback.clone(),
                end: clock.end.clone().unwrap_or_default(),
            },
            None => Self {
                start: EventKind::FeatureDefined.prd_name().into_owned(),
                fallback: Some(EventKind::TaskStarted.prd_name().into_owned()),
                end: template.final_event.clone(),
            },
        }
    }

    /// Whether `event` stops the clock.
    pub fn ends(&self, event: &Event) -> bool {
        event.kind.prd_name() == self.end
    }

    fn starts(&self, event: &Event) -> bool {
        event.kind.prd_name() == self.start
    }

    fn falls_back(&self, event: &Event) -> bool {
        self.fallback
            .as_deref()
            .is_some_and(|name| event.kind.prd_name() == name)
    }
}

/// The cycle of every feature and milestone, and of the mission.
#[derive(Clone, Debug)]
pub struct Cycles {
    pub features: BTreeMap<FeatureId, Cycle>,
    pub milestones: BTreeMap<MilestoneId, Cycle>,
    pub mission: Cycle,
    /// Features that had no `feature.defined`.
    pub fallback_features: usize,
}

/// `now` is the instant an open clock reads: the report's end.
pub fn cycles(events: &[Event], template: &CycleTemplate, now: Timestamp) -> Cycles {
    let clock = ClockKinds::of(template);
    let line = timeline(events);
    let ends: Vec<&Event> = line.iter().copied().filter(|e| clock.ends(e)).collect();
    let integrations: Vec<&Event> = line
        .iter()
        .copied()
        .filter(|e| e.kind == EventKind::IntegrationMerged)
        .collect();

    let ids: BTreeSet<&FeatureId> = events
        .iter()
        .filter_map(|e| e.feature_id.as_ref())
        .collect();
    let mut starts: BTreeMap<&FeatureId, (Timestamp, bool)> = BTreeMap::new();
    let mut homes: BTreeMap<&FeatureId, &MilestoneId> = BTreeMap::new();
    for id in ids {
        let own: Vec<&Event> = line
            .iter()
            .copied()
            .filter(|e| e.feature_id.as_ref() == Some(id))
            .collect();
        let found = own
            .iter()
            .find(|e| clock.starts(e))
            .map(|e| (e.at, false))
            .or_else(|| {
                own.iter()
                    .find(|e| clock.falls_back(e))
                    .map(|e| (e.at, true))
                    // No start event at all: the feature's first measured span begins the clock.
                    .or_else(|| {
                        own.iter()
                            .filter_map(|e| event_span(e).map(|span| span.from).or(Some(e.at)))
                            .min()
                            .map(|at| (at, true))
                    })
            });
        if let Some(found) = found {
            starts.insert(id, found);
        }
        if let Some(home) = own.iter().find_map(|e| e.milestone_id.as_ref()) {
            homes.insert(id, home);
        }
    }

    let mut features = BTreeMap::new();
    for (id, start) in &starts {
        let home = homes.get(id).copied();
        let end = ends
            .iter()
            .find(|e| e.at >= start.0 && e.feature_id.as_ref() == Some(*id))
            .or_else(|| {
                ends.iter().find(|e| {
                    e.at >= start.0
                        && e.feature_id.is_none()
                        && e.milestone_id.as_ref() == home
                        && home.is_some()
                })
            })
            .or_else(|| {
                ends.iter()
                    .find(|e| e.at >= start.0 && e.feature_id.is_none() && e.milestone_id.is_none())
            })
            .map(|e| (e.at, CycleEnd::HandedToCd));
        features.insert((*id).clone(), finish(Some(*start), end, now));
    }
    for id in events.iter().filter_map(|e| e.feature_id.as_ref()) {
        features
            .entry(id.clone())
            .or_insert_with(|| finish(None, None, now));
    }

    let milestone_ids: BTreeSet<&MilestoneId> = events
        .iter()
        .filter_map(|e| e.milestone_id.as_ref())
        .collect();
    let mut milestones = BTreeMap::new();
    for id in milestone_ids {
        // A fallback start (a feature the plan never defined) starts the milestone only when
        // the milestone has no `feature.defined` of its own.
        let own_starts = line
            .iter()
            .filter(|e| clock.starts(e) && e.milestone_id.as_ref() == Some(id))
            .map(|e| e.at)
            .min()
            .or_else(|| {
                starts
                    .iter()
                    .filter(|(feature, _)| homes.get(*feature).copied() == Some(id))
                    .map(|(_, start)| start.0)
                    .min()
            });
        let end = ends
            .iter()
            .rev()
            .find(|e| {
                e.milestone_id.as_ref() == Some(id)
                    || (e.milestone_id.is_none() && e.feature_id.is_none())
            })
            .map(|e| (e.at, CycleEnd::HandedToCd))
            .or_else(|| {
                integrations
                    .iter()
                    .rev()
                    .find(|e| e.milestone_id.as_ref() == Some(id))
                    .map(|e| (e.at, CycleEnd::Integration))
            });
        milestones.insert(
            id.clone(),
            finish(own_starts.map(|at| (at, false)), end, now),
        );
    }

    let mission_start = line
        .iter()
        .filter(|e| clock.starts(e))
        .map(|e| e.at)
        .chain(starts.values().map(|start| start.0))
        .min();
    let mission_end = ends.last().map(|e| (e.at, CycleEnd::HandedToCd));
    Cycles {
        fallback_features: features.values().filter(|cycle| cycle.fallback).count(),
        features,
        milestones,
        mission: finish(mission_start.map(|at| (at, false)), mission_end, now),
    }
}

/// Seconds a feature spent in a lane of the template: its first event there to its last.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StageTime {
    pub lane: LaneId,
    pub seconds: u64,
}

/// One stage per lane of the template, in its order; a lane the feature never entered is zero.
pub fn stage_times(events_of_feature: &[Event], template: &CycleTemplate) -> Vec<StageTime> {
    let line = timeline(events_of_feature);
    template
        .lanes
        .iter()
        .map(|lane| {
            let mut at = line.iter().filter(|e| e.lane == lane.id).map(|e| e.at);
            let first = at.next();
            let last = at.next_back().or(first);
            StageTime {
                lane: lane.id.clone(),
                seconds: first.zip(last).map_or(0, |(f, l)| whole_seconds(f, l)),
            }
        })
        .collect()
}

/// Stage times added up lane by lane.
pub fn add_stages(into: &mut [StageTime], from: &[StageTime]) {
    for (slot, add) in into.iter_mut().zip(from) {
        slot.seconds += add.seconds;
    }
}
