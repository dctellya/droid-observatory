//! The bottleneck: the lane and the role where features waited longest on a human, on
//! validation or behind another feature (architecture §9).

use std::time::Duration;

use serde::Serialize;

use super::span::timeline;
use super::state::{Interval, TimeState};
use crate::model::{CycleTemplate, Event, LaneId, Role};

/// Waiting time per lane of the template (by index) and per role (by `Role::ALL` index).
#[derive(Clone, Debug, Default)]
pub struct Waiting {
    lanes: Vec<Duration>,
    roles: Vec<Duration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Bottleneck {
    pub lane: LaneId,
    /// Whole seconds the card waited while its latest event sat in `lane`.
    pub waiting_s: u64,
    pub role: Role,
    pub role_waiting_s: u64,
}

fn is_waiting(state: TimeState) -> bool {
    matches!(
        state,
        TimeState::WaitingHuman(_) | TimeState::WaitingValidation | TimeState::IdleQueued
    )
}

impl Waiting {
    pub fn new(template: &CycleTemplate) -> Self {
        Self {
            lanes: vec![Duration::ZERO; template.lanes.len()],
            roles: vec![Duration::ZERO; Role::ALL.len()],
        }
    }

    /// Each waiting interval counts toward the lane and role of the latest event at or before
    /// its start (the first event when none precedes it): the place the card stopped.
    pub fn of_card(
        template: &CycleTemplate,
        events_of_feature: &[Event],
        intervals: &[Interval],
    ) -> Self {
        let mut found = Self::new(template);
        let line = timeline(events_of_feature);
        for interval in intervals.iter().filter(|i| is_waiting(i.state)) {
            let Some(at) = line
                .iter()
                .rev()
                .find(|event| event.at <= interval.from)
                .or_else(|| line.first())
            else {
                continue;
            };
            if let Some(index) = template.index(&at.lane) {
                found.lanes[index] += interval.length();
            }
            if let Some(index) = Role::ALL.iter().position(|role| *role == at.role) {
                found.roles[index] += interval.length();
            }
        }
        found
    }

    pub fn add(&mut self, other: &Waiting) {
        for (slot, add) in self.lanes.iter_mut().zip(&other.lanes) {
            *slot += *add;
        }
        for (slot, add) in self.roles.iter_mut().zip(&other.roles) {
            *slot += *add;
        }
    }

    /// The lane and role with the longest wait; the earlier one in the template's order (the
    /// roles' order) on a tie. `None` when nothing waited.
    pub fn bottleneck(&self, template: &CycleTemplate) -> Option<Bottleneck> {
        let longest = |times: &[Duration]| {
            times
                .iter()
                .enumerate()
                .filter(|(_, time)| !time.is_zero())
                .fold(
                    None::<(usize, Duration)>,
                    |best, (index, time)| match best {
                        Some((_, held)) if held >= *time => best,
                        _ => Some((index, *time)),
                    },
                )
        };
        let seconds = |time: Duration| u64::try_from((time.as_millis() + 500) / 1000).unwrap_or(0);
        let (lane, lane_wait) = longest(&self.lanes)?;
        let (role, role_wait) = longest(&self.roles)?;
        Some(Bottleneck {
            lane: template.lanes.get(lane)?.id.clone(),
            waiting_s: seconds(lane_wait),
            role: *Role::ALL.get(role)?,
            role_waiting_s: seconds(role_wait),
        })
    }
}
