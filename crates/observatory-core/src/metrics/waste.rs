//! Waste: measured as eight causes, displayed as five groups (F-028). `throttled` is neither.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::rework::{ReworkSpan, first_pass_and_rework};
use super::span::{Span, union};
use super::state::{Interval, TimeState, WaitKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasteCause {
    WaitingHumanQuestion,
    WaitingHumanPermission,
    WaitingMerge,
    WaitingValidation,
    Rework,
    RetryFailure,
    IdleQueued,
    IdleUnscheduled,
}

impl WasteCause {
    /// The eight causes in the order of the CSV columns.
    pub const ALL: [WasteCause; 8] = [
        Self::WaitingHumanQuestion,
        Self::WaitingHumanPermission,
        Self::WaitingMerge,
        Self::WaitingValidation,
        Self::Rework,
        Self::RetryFailure,
        Self::IdleQueued,
        Self::IdleUnscheduled,
    ];

    pub fn group(self) -> WasteGroup {
        match self {
            Self::WaitingHumanQuestion | Self::WaitingHumanPermission | Self::WaitingMerge => {
                WasteGroup::WaitingOnHuman
            }
            Self::WaitingValidation => WasteGroup::WaitingOnValidation,
            Self::Rework => WasteGroup::Rework,
            Self::RetryFailure => WasteGroup::Retry,
            Self::IdleQueued | Self::IdleUnscheduled => WasteGroup::Idle,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasteGroup {
    WaitingOnHuman,
    WaitingOnValidation,
    Rework,
    Retry,
    Idle,
}

impl WasteGroup {
    pub const ALL: [WasteGroup; 5] = [
        Self::WaitingOnHuman,
        Self::WaitingOnValidation,
        Self::Rework,
        Self::Retry,
        Self::Idle,
    ];
}

impl TimeState {
    /// The waste cause of the time spent in this state; Active is not waste (its share inside
    /// a rework window is the `rework` cause, which a state cannot tell).
    pub fn waste_cause(self) -> Option<WasteCause> {
        match self {
            Self::Active => None,
            Self::WaitingHuman(WaitKind::Question) => Some(WasteCause::WaitingHumanQuestion),
            Self::WaitingHuman(WaitKind::Permission) => Some(WasteCause::WaitingHumanPermission),
            Self::WaitingHuman(WaitKind::Merge) => Some(WasteCause::WaitingMerge),
            Self::WaitingValidation => Some(WasteCause::WaitingValidation),
            Self::RetryFailure => Some(WasteCause::RetryFailure),
            Self::IdleQueued => Some(WasteCause::IdleQueued),
            Self::IdleUnscheduled => Some(WasteCause::IdleUnscheduled),
        }
    }
}

/// Time per waste cause, and the bands of time it was measured in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WasteTotals {
    by_cause: BTreeMap<WasteCause, Duration>,
    bands: BTreeMap<WasteCause, Vec<Span>>,
}

impl WasteTotals {
    pub fn duration(&self, cause: WasteCause) -> Duration {
        self.by_cause.get(&cause).copied().unwrap_or_default()
    }

    /// The spans of time counted under `cause`, oldest first. The bands of one card's causes do
    /// not overlap: the intervals are disjoint and the `rework` band lies inside Active ones.
    pub fn bands(&self, cause: WasteCause) -> &[Span] {
        self.bands.get(&cause).map_or(&[], Vec::as_slice)
    }

    /// The cause whose band holds `at` (a band is half open: `from <= at < to`).
    pub fn cause_at(&self, at: crate::model::Timestamp) -> Option<WasteCause> {
        WasteCause::ALL.into_iter().find(|cause| {
            self.bands(*cause)
                .iter()
                .any(|band| band.from <= at && at < band.to)
        })
    }

    /// Whole seconds, rounded to the nearest.
    pub fn seconds(&self, cause: WasteCause) -> u64 {
        let millis = self.duration(cause).as_millis();
        u64::try_from((millis + 500) / 1000).unwrap_or(u64::MAX)
    }

    /// The sum of the seconds of the group's causes, so the groups add up to the causes.
    pub fn group_seconds(&self, group: WasteGroup) -> u64 {
        WasteCause::ALL
            .into_iter()
            .filter(|cause| cause.group() == group)
            .map(|cause| self.seconds(cause))
            .sum()
    }

    pub fn total_seconds(&self) -> u64 {
        WasteCause::ALL
            .into_iter()
            .map(|cause| self.seconds(cause))
            .sum()
    }
}

/// The waste of one card: the intervals' non-Active states by cause, and the Active time inside
/// `rework` spans as `rework`.
pub fn waste_by_cause(intervals: &[Interval], rework: &[ReworkSpan]) -> WasteTotals {
    let mut totals = WasteTotals::default();
    for interval in intervals {
        if let Some(cause) = interval.state.waste_cause() {
            *totals.by_cause.entry(cause).or_default() += interval.length();
            totals.bands.entry(cause).or_default().push(Span {
                from: interval.from,
                to: interval.to,
            });
        }
    }
    let (_, rework_time) = first_pass_and_rework(intervals, rework);
    if !rework_time.is_zero() {
        totals.by_cause.insert(WasteCause::Rework, rework_time);
        let covered = union(
            rework
                .iter()
                .map(|span| Span {
                    from: span.from,
                    to: span.to,
                })
                .collect(),
        );
        let band = totals.bands.entry(WasteCause::Rework).or_default();
        for interval in intervals.iter().filter(|i| i.state == TimeState::Active) {
            for span in &covered {
                let (from, to) = (span.from.max(interval.from), span.to.min(interval.to));
                if from < to {
                    band.push(Span { from, to });
                }
            }
        }
    }
    totals
}
