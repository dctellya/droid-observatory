//! `throttled` (F-033, D-028): the platform stopped the work. Measured from the pause marker
//! to the resume, reported apart from the eight waste causes and the five groups.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use super::notes::Notes;
use super::span::{Span, gap, timeline, union};
use crate::model::{Event, EventKind, MilestoneId, Timestamp};

/// The reason of a pause that records none.
pub const UNKNOWN_REASON: &str = "unknown";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ThrottledInterval {
    pub from: Timestamp,
    pub to: Timestamp,
    pub reason: String,
    /// Whole minutes, rounded to the nearest.
    pub minutes: u64,
    pub milestone: Option<MilestoneId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Throttled {
    pub intervals: Vec<ThrottledInterval>,
    /// The run: the minutes of the union of the intervals.
    pub mission_minutes: u64,
    pub per_milestone: BTreeMap<MilestoneId, u64>,
}

impl Throttled {
    pub fn spans(&self) -> Vec<Span> {
        self.intervals
            .iter()
            .map(|interval| Span {
                from: interval.from,
                to: interval.to,
            })
            .collect()
    }
}

pub(super) fn minutes(length: Duration) -> u64 {
    u64::try_from((length.as_millis() + 30_000) / 60_000).unwrap_or(u64::MAX)
}

/// A `mission.paused` and the next `mission.resumed` bound an interval, with the reason the
/// pause recorded. A capture with no resume marker (a session file) has the escalations
/// of a stop and unpaired pauses: the interval runs from the last event before them to the
/// first event after (`end` when none follows).
pub fn throttled(events: &[Event], notes: &Notes, end: Timestamp) -> Throttled {
    let line = timeline(events);
    let is_marker = |event: &Event| {
        matches!(
            event.kind,
            EventKind::EscalationRaised | EventKind::MissionPaused
        )
    };
    let mut intervals = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let event = line[i];
        let resume = (i + 1..line.len())
            .take_while(|&j| line[j].kind != EventKind::MissionPaused)
            .find(|&j| line[j].kind == EventKind::MissionResumed);
        if event.kind == EventKind::MissionPaused
            && let Some(j) = resume
        {
            let reason = notes
                .pause_reason(event)
                .or_else(|| notes.pause_reason(line[j]));
            intervals.push(interval(event.at, line[j].at, reason, event));
            i = j + 1;
        } else if is_marker(event) {
            let mut last = i;
            while last + 1 < line.len() && is_marker(line[last + 1]) {
                last += 1;
            }
            let before = i.checked_sub(1).map(|b| line[b]);
            let from = before.map_or(event.at, |b| b.at);
            let to = line.get(last + 1).map_or(end, |next| next.at);
            let reason = line[i..=last].iter().find_map(|m| notes.pause_reason(m));
            let named = event.milestone_id.is_some().then_some(event).or(before);
            if from < to {
                intervals.push(interval(from, to, reason, named.unwrap_or(event)));
            }
            i = last + 1;
        } else {
            i += 1;
        }
    }

    let mut per_milestone: BTreeMap<MilestoneId, Duration> = BTreeMap::new();
    for found in &intervals {
        if let Some(milestone) = &found.milestone {
            *per_milestone.entry(milestone.clone()).or_default() += gap(found.from, found.to);
        }
    }
    let covered: Duration = union(
        intervals
            .iter()
            .map(|found| Span {
                from: found.from,
                to: found.to,
            })
            .collect(),
    )
    .iter()
    .map(Span::length)
    .sum();
    Throttled {
        mission_minutes: minutes(covered),
        per_milestone: per_milestone
            .into_iter()
            .map(|(milestone, length)| (milestone, minutes(length)))
            .collect(),
        intervals,
    }
}

fn interval(
    from: Timestamp,
    to: Timestamp,
    reason: Option<&str>,
    named: &Event,
) -> ThrottledInterval {
    ThrottledInterval {
        from,
        to,
        reason: reason.unwrap_or(UNKNOWN_REASON).to_owned(),
        minutes: minutes(gap(from, to)),
        milestone: named.milestone_id.clone(),
    }
}
