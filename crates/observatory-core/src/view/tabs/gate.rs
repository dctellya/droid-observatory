//! Gate (FR-D5): the mission PR, its reviews, checks and attestations, the merge policy and the
//! rule that decided it, the merge history and the two clocks of F-028.

use crate::model::{Event, EventId, EventKind, SourcePtr, Timestamp};
use crate::view::{GateCheckRow, GateClockLine, GateGroupView};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrLink {
    pub number: u64,
    /// Where the PR was observed (the API URL of the `pr.opened` event), when known.
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryRow {
    pub event_id: EventId,
    pub at: Timestamp,
    pub text: String,
}

/// A clock the template names for the final lane: its label, and its value once computed. The
/// lead time to production is the delivery platform's clock and is named, never computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateClock {
    pub label: String,
    pub value: Option<String>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateTab {
    /// `Pending` until a PR exists.
    pub state: String,
    pub pr: Option<PrLink>,
    pub reviews: u64,
    pub reviews_text: String,
    pub checks: Vec<GateCheckRow>,
    pub attestations: Vec<String>,
    /// The merge policy and the rule that decided it, as the last poll read them.
    pub policy_text: String,
    pub history: Vec<HistoryRow>,
    pub clocks: Vec<GateClock>,
}

pub fn gate_tab(view: &GateGroupView, events: &[Event]) -> GateTab {
    let url = events
        .iter()
        .filter(|event| event.kind == EventKind::PrOpened)
        .find_map(|event| match &event.source {
            SourcePtr::GitHub { url } => Some(url.clone()),
            _ => None,
        });
    let mut history: Vec<HistoryRow> = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::HandedToCd { .. }))
        .map(|event| HistoryRow {
            event_id: event.id.clone(),
            at: event.at,
            text: view
                .handed_to_cd
                .clone()
                .unwrap_or_else(|| "handed to CD".to_owned()),
        })
        .collect();
    history.sort_by_key(|row| row.at);
    GateTab {
        state: match view.pr_number {
            None if view.badge_text == "pending" => "Pending".to_owned(),
            _ => view.badge_text.clone(),
        },
        pr: view.pr_number.map(|number| PrLink { number, url }),
        reviews: view.reviews,
        reviews_text: view.reviews_text.clone(),
        checks: view.checks.clone(),
        attestations: view.attestations.clone(),
        policy_text: view.ruleset_text.clone(),
        history,
        clocks: view
            .clocks
            .iter()
            .map(|GateClockLine { label, note }| GateClock {
                label: label.clone(),
                value: None,
                note: note.clone(),
            })
            .collect(),
    }
}
