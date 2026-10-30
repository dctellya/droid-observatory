//! The five Details tabs beside the Timeline (FR-D2 to FR-D6): each is a function over the events
//! held, so the browser host and the Tauri window draw the same figures.

mod commits;
mod cost;
mod criteria;
mod gate;
mod review;

pub use commits::{CommitRow, CommitsTab, EventRow, commits_tab};
pub use cost::{CostTab, ReworkRow, UsageRow, WasteRow, cost_tab};
pub use criteria::{CriteriaTab, CriterionRow, FeatureCriteria, criteria_tab};
pub use gate::{GateClock, GateTab, HistoryRow, PrLink, gate_tab};
pub use review::{FindingRow, ReviewTab, ScenarioRow, review_tab};

use crate::model::{Event, MilestoneId, Timestamp};

/// The events of `milestone` that name it, without the Panel's `turn.usage`, oldest first.
fn milestone_events<'a>(events: &'a [Event], milestone: &MilestoneId) -> Vec<&'a Event> {
    let mut found: Vec<&Event> = events
        .iter()
        .filter(|event| event.milestone_id.as_ref() == Some(milestone))
        .filter(|event| !event.kind.is_panel_only())
        .collect();
    found.sort_by_key(|event| event.at);
    found
}

/// The instant the tabs are made at: the last event of the capture, so the same capture gives
/// the same figures on any day.
pub(super) fn tabs_end(events: &[Event]) -> Option<Timestamp> {
    events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .max()
}
